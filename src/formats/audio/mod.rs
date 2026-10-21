pub mod envelope;
pub mod seq;
pub mod midi;
pub mod sf2;
pub mod vab_sf2;

#[cfg(test)]
pub mod fixtures;

pub use seq::{Seq, SeqHeader, SeqEvent, SeqEventType, read_seq, split_seq_bank};
pub use midi::{
    no_bend_ranges, BendRanges, convert_seq_bank_to_midi, convert_seq_to_midi, pitch_bend_ranges,
    MIDI_LOOP_START_MARKER, MIDI_LOOP_END_MARKER,
};
pub use envelope::{approximate_envelope, decode_adsr, SpuAdsr, VolumeEnvelope};
pub use sf2::{SoundFont, Sf2Sample, Sf2Instrument, Sf2Zone, Sf2Generator, Sf2GeneratorValue, Sf2Preset, write_sf2};
pub use vab_sf2::{VabSoundFont, convert_vab_to_soundfont};
