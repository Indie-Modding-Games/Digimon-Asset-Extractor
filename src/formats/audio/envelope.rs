//! The sound chip's ADSR envelope (the two 16-bit registers a tone carries) and its approximation as a SoundFont
//! volume envelope.

const SAMPLE_RATE: f64 = 44100.0;
const MAX_LEVEL: i32 = 0x7FFF;
const SUSTAIN_LEVEL_UNIT: i32 = 0x800;
const EXPONENTIAL_SLOWDOWN_LEVEL: i32 = 0x6000;
const UNIT_SHIFT: u32 = 11;
const RISE_BASE: i32 = 7;
const FALL_BASE: i32 = -8;
const FROZEN_SUSTAIN_RATE: u8 = 0x7F;
const MIN_TIMECENTS: i16 = -12000;
const MAX_TIMECENTS: i16 = 8000;
const SILENCE_CENTIBELS: i16 = 1000;
const MAX_CENTIBELS: f64 = 1440.0;
const FULL_FALL_DECIBELS: f64 = 100.0;
const MEASURE_FALL_RATIO: i32 = 1000;
const MAX_STEPS: usize = 1 << 20;

/// The fields of the two registers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SpuAdsr {
    pub attack_exponential: bool,
    pub attack_shift: u8, // 0-31
    pub attack_step: u8,  // 0-3: the level rises by 7 - step
    pub decay_shift: u8,  // 0-15; the decay is exponential and falls by 8
    pub sustain_level: u8, // 0-15: (n + 1) * 0x800
    pub sustain_exponential: bool,
    pub sustain_decrease: bool,
    pub sustain_shift: u8, // 0-31
    pub sustain_step: u8,  // 0-3: the level rises by 7 - step or falls by 8 - step
    pub release_exponential: bool,
    pub release_shift: u8, // 0-31; the release falls by 8
}

/// The SoundFont volume envelope that comes closest.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VolumeEnvelope {
    pub attack: i16,    // timecents
    pub decay: i16,     // timecents for a 100 dB fall
    pub sustain: i16,   // centibels below the peak
    pub release: i16,   // timecents for a 100 dB fall
    pub sloped_sustain: bool, // the chip's sustain changes level; a SoundFont sustain cannot
}

/// One phase of the envelope: how fast the level moves and how long it waits.
struct Phase {
    shift: u32,
    step: i32,
    exponential: bool,
    decreasing: bool,
}

/// How long a phase took and where the level ended.
struct PhaseRun {
    samples: u64,
    level: i32,
}

/// Steps a phase from a level until it reaches the target, and returns how long that took.
fn run_phase(phase: &Phase, start: i32, target: i32) -> PhaseRun {
    let slower = phase.shift.saturating_sub(UNIT_SHIFT);
    let larger = UNIT_SHIFT.saturating_sub(phase.shift);
    let base_step = phase.step * (1 << larger);
    let mut samples = 0u64;
    let mut level = start;
    for _ in 0..MAX_STEPS {
        let reached = if phase.decreasing {
            level <= target
        } else {
            level >= target
        };
        if reached {
            break;
        }
        let mut cycles = 1u64 << slower;
        let mut step = base_step;
        if phase.exponential && !phase.decreasing && level > EXPONENTIAL_SLOWDOWN_LEVEL {
            cycles *= 4;
        }
        if phase.exponential && phase.decreasing {
            step = (step * level) >> 15;
        }
        level = (level + step).clamp(0, MAX_LEVEL);
        samples += cycles;
    }
    PhaseRun { samples, level }
}

fn timecents(seconds: f64) -> i16 {
    if seconds <= 0.0 {
        return MIN_TIMECENTS;
    }
    let value = (1200.0 * seconds.log2()).round();
    value.clamp(MIN_TIMECENTS as f64, MAX_TIMECENTS as f64) as i16
}

fn seconds_of(run: &PhaseRun) -> f64 {
    run.samples as f64 / SAMPLE_RATE
}

/// The time of a 100 dB fall at the rate a falling run from `from` showed.
fn fall_timecents(run: &PhaseRun, from: i32) -> i16 {
    if run.level <= 0 {
        return timecents(seconds_of(run));
    }
    let decibels = 20.0 * (from as f64 / run.level as f64).log10();
    if decibels <= 0.0 {
        return MIN_TIMECENTS;
    }
    timecents(seconds_of(run) * FULL_FALL_DECIBELS / decibels)
}

/// The time of a falling phase from a level: scaled to 100 dB when exponential, to silence when linear.
fn fall_time(phase: &Phase, from: i32) -> i16 {
    if phase.exponential {
        return fall_timecents(
            &run_phase(phase, from, std::cmp::max(1, from / MEASURE_FALL_RATIO)),
            from,
        );
    }
    timecents(seconds_of(&run_phase(phase, from, 0)))
}

/// Splits the registers into their fields.
pub fn decode_adsr(adsr1: u16, adsr2: u16) -> SpuAdsr {
    let bits = |value: u16, shift: u32, mask: u16| ((value >> shift) & mask) as u8;
    SpuAdsr {
        attack_exponential: (adsr1 & 0x8000) != 0,
        attack_shift: bits(adsr1, 10, 0x1F),
        attack_step: bits(adsr1, 8, 0x03),
        decay_shift: bits(adsr1, 4, 0x0F),
        sustain_level: bits(adsr1, 0, 0x0F),
        sustain_exponential: (adsr2 & 0x8000) != 0,
        sustain_decrease: (adsr2 & 0x4000) != 0,
        sustain_shift: bits(adsr2, 8, 0x1F),
        sustain_step: bits(adsr2, 6, 0x03),
        release_exponential: (adsr2 & 0x0020) != 0,
        release_shift: bits(adsr2, 0, 0x1F),
    }
}

/// Approximates the chip's envelope by stepping it at 44100 Hz, as the chip does: a phase waits
/// `1 << max(0, shift - 11)` samples per step and moves by `step << max(0, 11 - shift)`; an exponential rise waits 4
/// times longer above 0x6000 and an exponential fall scales its step by level / 0x8000. The attack is the time from 0
/// to 0x7FFF. The decay and an exponential release fall at a constant rate in decibels, as a SoundFont decay and
/// release do, so their time is scaled to a 100 dB fall; a linear release takes its time from 0x7FFF to 0. A falling
/// sustain slope (any rate but 0x7F) becomes the decay, towards silence; a rising one is ignored. Both are marked as
/// sloped.
pub fn approximate_envelope(adsr1: u16, adsr2: u16) -> VolumeEnvelope {
    let adsr = decode_adsr(adsr1, adsr2);
    let mut envelope = VolumeEnvelope::default();

    let attack = Phase {
        shift: adsr.attack_shift as u32,
        step: RISE_BASE - adsr.attack_step as i32,
        exponential: adsr.attack_exponential,
        decreasing: false,
    };
    envelope.attack = timecents(seconds_of(&run_phase(&attack, 0, MAX_LEVEL)));

    let sustain_level = ((adsr.sustain_level as i32 + 1) * SUSTAIN_LEVEL_UNIT).min(MAX_LEVEL);
    envelope.sustain = (200.0 * (MAX_LEVEL as f64 / sustain_level as f64).log10())
        .round()
        .clamp(0.0, MAX_CENTIBELS) as i16;
    let decay = Phase {
        shift: adsr.decay_shift as u32,
        step: FALL_BASE,
        exponential: true,
        decreasing: true,
    };
    envelope.decay = if sustain_level >= MAX_LEVEL {
        MIN_TIMECENTS
    } else {
        fall_time(&decay, MAX_LEVEL)
    };

    let sustain_rate = (adsr.sustain_shift << 2) | adsr.sustain_step;
    envelope.sloped_sustain = sustain_rate != FROZEN_SUSTAIN_RATE;
    if envelope.sloped_sustain && adsr.sustain_decrease {
        let sustain = Phase {
            shift: adsr.sustain_shift as u32,
            step: FALL_BASE + adsr.sustain_step as i32,
            exponential: adsr.sustain_exponential,
            decreasing: true,
        };
        envelope.decay = fall_time(&sustain, sustain_level);
        envelope.sustain = SILENCE_CENTIBELS;
    }

    let release = Phase {
        shift: adsr.release_shift as u32,
        step: FALL_BASE,
        exponential: adsr.release_exponential,
        decreasing: true,
    };
    envelope.release = fall_time(&release, MAX_LEVEL);
    envelope
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expected_timecents(samples: f64) -> i16 {
        (1200.0 * (samples / 44100.0).log2()).round() as i16
    }

    #[test]
    fn the_registers_hold_the_fields_of_the_envelope() {
        assert_eq!(
            decode_adsr(0x80FF, 0x5FC0),
            SpuAdsr {
                attack_exponential: true,
                attack_shift: 0,
                attack_step: 0,
                decay_shift: 15,
                sustain_level: 15,
                sustain_exponential: false,
                sustain_decrease: true,
                sustain_shift: 31,
                sustain_step: 3,
                release_exponential: false,
                release_shift: 0,
            }
        );
    }

    #[test]
    fn a_linear_attack_of_shift_twenty_waits_five_hundred_and_twelve_samples_per_step_of_seven() {
        let envelope = approximate_envelope(0x500F, 0x1FC0 | 20);
        assert_eq!(envelope.attack, expected_timecents(4681.0 * 512.0));
        assert_eq!(envelope.decay, -12000); // the sustain level 15 is the peak: no decay
        assert_eq!(envelope.sustain, 0);
        assert_eq!(envelope.release, expected_timecents(4096.0 * 512.0)); // linear: 0x7FFF / 8 steps
        assert!(!envelope.sloped_sustain);
    }

    #[test]
    fn the_fast_phases_clamp_to_the_shortest_time() {
        let envelope = approximate_envelope(0x000F, 0x1FC0);
        assert_eq!(envelope.attack, -12000);
        assert_eq!(envelope.release, -12000);
    }

    #[test]
    fn the_sustain_level_is_in_centibels_below_the_peak() {
        assert_eq!(approximate_envelope(0x0000, 0x1FC0).sustain, 241);
        assert_eq!(approximate_envelope(0x0007, 0x1FC0).sustain, 60);
    }

    #[test]
    fn an_exponential_attack_is_slower_than_a_linear_one_and_a_longer_decay_shift_decays_slower() {
        assert!(
            approximate_envelope(0xD000, 0x1FC0).attack > approximate_envelope(0x5000, 0x1FC0).attack
        );
        assert!(
            approximate_envelope(0x00F0, 0x1FC0).decay > approximate_envelope(0x0050, 0x1FC0).decay
        );
        assert!(
            approximate_envelope(0x0000, 0x1FC0 | 0x20 | 20).release
                > approximate_envelope(0x0000, 0x1FC0 | 0x20 | 10).release
        );
    }

    #[test]
    fn a_falling_sustain_slope_becomes_the_decay_towards_silence() {
        let envelope = approximate_envelope(0x000F, 0xC000 | (20 << 8));
        assert!(envelope.sloped_sustain);
        assert_eq!(envelope.sustain, 1000);
        assert!(envelope.decay > 0);

        let envelope = approximate_envelope(0x0007, 20 << 8);
        assert!(envelope.sloped_sustain);
        assert_eq!(envelope.sustain, 60);
    }
}
