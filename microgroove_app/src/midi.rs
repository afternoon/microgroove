use defmt::{debug, trace};
use midi_types::MidiMessage;

pub fn log_message(message: &MidiMessage) {
    match message {
        MidiMessage::TimingClock => trace!("[midi_send] clock"),
        MidiMessage::Start => trace!("[midi_send] start"),
        MidiMessage::Stop => trace!("[midi_send] stop"),
        MidiMessage::Continue => trace!("[midi_send] continue"),
        MidiMessage::NoteOn(midi_channel, note, velocity) => {
            let midi_channel: u8 = (*midi_channel).into();
            let note: u8 = (*note).into();
            let velocity: u8 = (*velocity).into();
            debug!(
                "[midi_send] note on midi_channel={} note={} velocity={}",
                midi_channel, note, velocity
            );
        }
        MidiMessage::NoteOff(midi_channel, note, _velocity) => {
            let midi_channel: u8 = (*midi_channel).into();
            let note: u8 = (*note).into();
            debug!(
                "[midi_send] note off midi_channel={} note={}",
                midi_channel, note
            );
        }
        _ => trace!("[midi_send] UNKNOWN"),
    }
}
