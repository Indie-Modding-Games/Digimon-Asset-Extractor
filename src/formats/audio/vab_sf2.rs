use crate::error::{DwExtractError, Result};
use crate::formats::audio::envelope::{approximate_envelope, VolumeEnvelope};
use crate::formats::audio::sf2::{SoundFont, Sf2Sample, Sf2Instrument, Sf2Zone, Sf2Generator, Sf2GeneratorValue, Sf2Preset, SampleLoop};
use crate::formats::vag::DecodedSample;
use crate::formats::vab2::{Vab, ToneAttributes};

const FULL_VOLUME: f64 = 127.0;
const MAX_ATTENUATION: f64 = 1440.0;
const PAN_CENTRE: i32 = 64;
const PAN_RANGE: i32 = 500;
const REVERB_ON: i16 = 1000;
const SOUND_FONT_RATE: u32 = 44100;
const MAX_KEY: u8 = 127;
const LOOP_CONTINUOUSLY: i16 = 1;

/// The envelope of a tone, cached by its two registers.
struct EnvelopeCache {
    entries: std::collections::HashMap<(u16, u16), VolumeEnvelope>,
}

impl EnvelopeCache {
    fn new() -> Self {
        Self {
            entries: std::collections::HashMap::new(),
        }
    }

    fn envelope_of(&mut self, tone: &ToneAttributes) -> VolumeEnvelope {
        let key = (tone.adsr1, tone.adsr2);
        *self
            .entries
            .entry(key)
            .or_insert_with(|| approximate_envelope(tone.adsr1, tone.adsr2))
    }
}

fn tone_attenuation(tone_volume: u8, program_volume: u8, bank_volume: u8) -> i16 {
    let ratio = (tone_volume as f64 / FULL_VOLUME) * (program_volume as f64 / FULL_VOLUME) * (bank_volume as f64 / FULL_VOLUME);
    if ratio <= 0.0 {
        return MAX_ATTENUATION as i16;
    }
    (-200.0 * ratio.log10()).round().clamp(0.0, MAX_ATTENUATION) as i16
}

fn tone_pan(tone_pan: u8, program_pan: u8) -> i16 {
    let offset = ((tone_pan as i32 - PAN_CENTRE) + (program_pan as i32 - PAN_CENTRE)).clamp(-PAN_CENTRE, PAN_CENTRE);
    (offset * PAN_RANGE / PAN_CENTRE) as i16
}

fn tone_fine_tune(shift: u8) -> i16 {
    (-(shift as f64 * 100.0 / 128.0).round()) as i16
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VabSoundFont {
    pub font: SoundFont,
    pub sloped_sustains: usize,
}

fn zone_of(
    vab: &Vab,
    program: usize,
    tone: &ToneAttributes,
    envelope: &VolumeEnvelope,
    sample: &DecodedSample,
    sample_index: usize,
) -> Sf2Zone {
    let program_attrs = &vab.programs[program];
    let mut generators = vec![
        Sf2GeneratorValue { generator: Sf2Generator::InitialAttenuation, amount: tone_attenuation(tone.vol, program_attrs.mvol, vab.header.mvol) },
        Sf2GeneratorValue { generator: Sf2Generator::Pan, amount: tone_pan(tone.pan, program_attrs.mpan) },
        Sf2GeneratorValue { generator: Sf2Generator::ReverbEffectsSend, amount: if (tone.mode & 0x04) != 0 { REVERB_ON } else { 0 } },
        Sf2GeneratorValue { generator: Sf2Generator::AttackVolEnv, amount: envelope.attack },
        Sf2GeneratorValue { generator: Sf2Generator::DecayVolEnv, amount: envelope.decay },
        Sf2GeneratorValue { generator: Sf2Generator::SustainVolEnv, amount: envelope.sustain },
        Sf2GeneratorValue { generator: Sf2Generator::ReleaseVolEnv, amount: envelope.release },
        Sf2GeneratorValue { generator: Sf2Generator::FineTune, amount: tone_fine_tune(tone.shift) },
        Sf2GeneratorValue { generator: Sf2Generator::OverridingRootKey, amount: tone.center.min(MAX_KEY) as i16 },
    ];

    if sample.loop_info.is_some() {
        generators.push(Sf2GeneratorValue {
            generator: Sf2Generator::SampleModes,
            amount: LOOP_CONTINUOUSLY,
        });
    }

    let key_high = tone.max.min(MAX_KEY);
    Sf2Zone {
        key_low: tone.min.min(key_high),
        key_high,
        generators,
        sample: sample_index,
    }
}

pub fn convert_vab_to_soundfont(
    vab: &Vab, 
    samples: &[DecodedSample],
    name: &str,
) -> Result<VabSoundFont> {
    let mut result = VabSoundFont::default();
    result.font.name = name.to_string();

    let mut sample_indices = std::collections::HashMap::new();
    let mut envelopes = EnvelopeCache::new();

    for block in &vab.tone_blocks {
        let program = block.program;
        let mut instrument = Sf2Instrument {
            name: format!("program_{:03}", program),
            zones: Vec::new(),
        };

        for tone in vab.tones_of(program).unwrap_or_default() {
            let number = tone.vag as usize;
            if number >= samples.len() || samples[number].pcm.is_empty() {
                return Err(DwExtractError::InvalidFormat(format!(
                    "VAB program {} plays sample {}, which has not been decoded", program, number
                )));
            }

            let sample = &samples[number];
            let sample_index = *sample_indices.entry(number).or_insert_with(|| {
                let idx = result.font.samples.len();
                result.font.samples.push(Sf2Sample {
                    name: format!("vag_{:03}", number),
                    pcm: sample.pcm.clone(),
                    sample_rate: SOUND_FONT_RATE,
                    loop_info: sample.loop_info.as_ref().map(|loop_info| SampleLoop {
                        start: loop_info.start,
                        end: loop_info.end,
                    }),
                    original_pitch: tone.center.min(MAX_KEY),
                    pitch_correction: 0,
                });
                idx
            });

            let envelope = envelopes.envelope_of(tone);
            if envelope.sloped_sustain {
                result.sloped_sustains += 1;
            }
            instrument.zones.push(zone_of(vab, program, tone, &envelope, sample, sample_index));
        }

        result.font.presets.push(Sf2Preset {
            name: instrument.name.clone(),
            preset: program as u16,
            bank: 0,
            instrument: result.font.instruments.len(),
        });
        result.font.instruments.push(instrument);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::audio::fixtures::{adpcm_sample, build_vab, ProgramSpec, ToneSpec, END, LOOP_START, REPEAT};
    use crate::formats::vab2::Vab;
    use std::io::Cursor;

    #[test]
    fn test_tone_attenuation() {
        assert_eq!(tone_attenuation(127, 127, 127), 0);
        assert_eq!(tone_attenuation(0, 127, 127), 1440);
    }

    #[test]
    fn test_tone_pan() {
        assert_eq!(tone_pan(64, 64), 0);
        assert_eq!(tone_pan(0, 64), -500);
        assert_eq!(tone_pan(127, 64), 492);
    }

    #[test]
    fn test_tone_fine_tune() {
        assert_eq!(tone_fine_tune(0), 0);
        assert_eq!(tone_fine_tune(64), -50);
        assert_eq!(tone_fine_tune(32), -25);
    }

    /// The amount of a generator of a zone.
    fn generator(zone: &Sf2Zone, which: Sf2Generator) -> Option<i16> {
        zone.generators
            .iter()
            .find(|value| value.generator == which)
            .map(|value| value.amount)
    }

    /// A sample decoded from the bytes of its blocks.
    fn decode_sample(bytes: &[u8]) -> DecodedSample {
        let decoded = crate::formats::adpcm::decode_adpcm(bytes);
        DecodedSample {
            pcm: decoded.pcm,
            loop_info: decoded.loop_info,
        }
    }

    #[test]
    fn a_preset_per_program_a_zone_per_tone_and_a_sample_per_played_vag() {
        let programs = vec![
            ProgramSpec {
                slot: 0,
                tones: vec![
                    ToneSpec {
                        min: 0,
                        max: 59,
                        vag: 1,
                        ..ToneSpec::default()
                    },
                    ToneSpec {
                        min: 60,
                        max: 72,
                        vag: 1,
                        ..ToneSpec::default()
                    },
                ],
                mvol: 127,
                mpan: 64,
            },
            ProgramSpec {
                slot: 4,
                tones: vec![ToneSpec {
                    center: 48,
                    shift: 64,
                    mode: 4,
                    vag: 2,
                    ..ToneSpec::default()
                }],
                mvol: 127,
                mpan: 64,
            },
        ];
        let vab_bytes = build_vab(&programs, &[32, 48], 2);
        let vab = Vab::read(&mut Cursor::new(vab_bytes.bytes())).unwrap();

        let mut samples = vec![
            DecodedSample::default(),
            DecodedSample::default(),
            DecodedSample::default(),
        ];
        samples[1] = decode_sample(&adpcm_sample(&[0, END]));
        samples[2] = decode_sample(&adpcm_sample(&[0, LOOP_START, END | REPEAT]));

        let converted = convert_vab_to_soundfont(&vab, &samples, "bank").unwrap();
        let font = &converted.font;
        assert_eq!(font.presets.len(), 2);
        assert_eq!(font.presets[1].preset, 4);
        assert_eq!(font.presets[1].instrument, 1);
        assert_eq!(font.samples.len(), 2); // the VAG 1 is shared by two tones
        assert_eq!(
            font.samples[1].loop_info,
            Some(SampleLoop {
                start: 28,
                end: 84
            })
        );
        assert_eq!(font.samples[1].original_pitch, 48);
        assert_eq!(font.instruments[0].zones.len(), 2);
        assert_eq!(font.instruments[0].zones[1].key_low, 60);
        assert_eq!(font.instruments[0].zones[1].key_high, 72);

        let zone = &font.instruments[1].zones[0];
        assert_eq!(generator(zone, Sf2Generator::OverridingRootKey), Some(48));
        assert_eq!(generator(zone, Sf2Generator::FineTune), Some(-50));
        assert_eq!(generator(zone, Sf2Generator::ReverbEffectsSend), Some(1000));
        assert_eq!(generator(zone, Sf2Generator::SampleModes), Some(1));

        assert_eq!(converted.sloped_sustains, 0);
        assert!(crate::formats::audio::sf2::write_sf2(font).is_ok());

        samples[2] = DecodedSample::default();
        assert!(convert_vab_to_soundfont(&vab, &samples, "bank").is_err());
    }
}
