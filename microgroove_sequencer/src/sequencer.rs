use embedded_midi::MidiMessage;
use fugit::{ExtU64, MicrosDurationU64};
use heapless::{HistoryBuffer, Vec};

use crate::{TimeDivision, Track, TRACK_COUNT};

// TODO will cause issues if polyphony
const MAX_MESSAGES_PER_TICK: usize = TRACK_COUNT * 2;

const MIDI_HISTORY_SAMPLE_COUNT: usize = 6;

#[derive(Debug)]
pub enum SequencerError {
    EnableTrackError(),
}

#[derive(Debug)]
pub enum ScheduledMidiMessage {
    Immediate(MidiMessage),
    Delayed(MidiMessage, MicrosDurationU64),
}

const DEFAULT_BPM: u64 = 130;
const DEFAULT_TICK_DURATION_US: u64 = (60 / DEFAULT_BPM) / 24;

pub struct Sequencer {
    pub tracks: Vec<Option<Track>, TRACK_COUNT>,
    pub tick: u32,
    playing: bool,
    last_tick_instant_us: Option<u64>,
    midi_tick_history: HistoryBuffer<u64, MIDI_HISTORY_SAMPLE_COUNT>,
}

impl Default for Sequencer {
    fn default() -> Sequencer {
        // create a set of empty tracks
        let mut tracks = Vec::new();
        for _ in 0..TRACK_COUNT {
            tracks
                .push(None)
                .expect("inserting track into tracks vector should succeed");
        }
        Sequencer {
            tracks,
            tick: 0,
            playing: false,
            last_tick_instant_us: None,
            midi_tick_history: HistoryBuffer::<u64, MIDI_HISTORY_SAMPLE_COUNT>::new(),
        }
    }
}

impl Sequencer {
    pub fn is_playing(&self) -> bool {
        self.playing
    }

    pub fn start_playing(&mut self) {
        self.tick = 0;
        self.playing = true
    }

    pub fn stop_playing(&mut self) {
        self.playing = false;
    }

    pub fn continue_playing(&mut self) {
        self.playing = true
    }

    pub fn enable_track(&mut self, track_num: u8, new_track: Track) -> &mut Track {
        self.tracks[track_num as usize].insert(new_track)
    }

    pub fn advance(&mut self, now_us: u64) -> Vec<ScheduledMidiMessage, MAX_MESSAGES_PER_TICK> {
        let tick_duration = self.average_tick_duration(now_us);

        let mut output_messages = Vec::new();

        if !self.playing {
            return output_messages;
        }

        for track in &self.tracks {
            if let Some(track) = track {
                if let Some(step) = track.step_at_tick(self.tick) {
                    let note_on_message =
                        MidiMessage::NoteOn(track.midi_channel, step.note, step.velocity);
                    output_messages
                        .push(ScheduledMidiMessage::Immediate(note_on_message))
                        .unwrap();

                    let note_off_message =
                        MidiMessage::NoteOff(track.midi_channel, step.note, 0.into());
                    let note_off_time = ((tick_duration.to_micros()
                        * (TimeDivision::division_length_24ppqn(track.time_division) as u64)
                        * step.length_step_cents as u64)
                        / 100)
                        .micros();
                    output_messages
                        .push(ScheduledMidiMessage::Delayed(
                            note_off_message,
                            note_off_time,
                        ))
                        .unwrap();
                }
            }
        }

        self.tick += 1;

        output_messages
    }

    /// Calculate average time between last k MIDI ticks. Defaults to tick frequency of
    /// 19,230ms, which is equivalent to 130BPM.
    fn average_tick_duration(&mut self, now_us: u64) -> MicrosDurationU64 {
        let mut tick_duration = DEFAULT_TICK_DURATION_US.micros();

        if let Some(last_tick_instant_us) = self.last_tick_instant_us {
            let last_tick_duration = now_us - last_tick_instant_us;
            self.midi_tick_history.write(last_tick_duration);
            tick_duration = (self.midi_tick_history.as_slice().iter().sum::<u64>()
                / self.midi_tick_history.len() as u64)
                .micros();
        }

        self.last_tick_instant_us = Some(now_us);

        tick_duration
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{machine_resources::MachineResources, sequence_generator::SequenceGenerator};

    #[test]
    fn sequencer_default_should_have_empty_tracks() {
        let sequencer = Sequencer::default();
        assert!(sequencer.tracks.iter().all(|track| track.is_none()));
    }

    #[test]
    fn sequencer_enable_track_should_insert_new_track() {
        let mut sequencer = Sequencer::default();
        let generator = SequenceGenerator::default();
        let mut machine_resources = MachineResources::new();
        let mut new_track = Track::default();
        new_track.sequence = generator.generate(new_track.length, &mut machine_resources);
        sequencer.enable_track(0, new_track);
        assert!(sequencer.tracks[0].is_some());
        assert!(sequencer.tracks[1..TRACK_COUNT]
            .iter()
            .all(|track| track.is_none()));
    }

    #[test]
    fn sequencer_should_start_stop_and_continue_playing() {
        let mut sequencer = Sequencer::default();
        assert_eq!(false, sequencer.is_playing());
        assert_eq!(0, sequencer.tick);
        sequencer.start_playing();
        assert_eq!(true, sequencer.is_playing());

        sequencer.advance(1);
        sequencer.stop_playing();
        assert_eq!(false, sequencer.is_playing());

        sequencer.advance(1); // should be ignored because sequencer stopped
        sequencer.continue_playing();
        sequencer.advance(1);
        assert_eq!(true, sequencer.is_playing());
        assert_eq!(2, sequencer.tick);

        sequencer.stop_playing();
        assert_eq!(2, sequencer.tick);

        sequencer.start_playing();
        assert_eq!(true, sequencer.is_playing());
        assert_eq!(0, sequencer.tick);
    }

    #[test]
    fn sequencer_should_calculate_average_tick_duration() {
        let mut sequencer = Sequencer::default();
        let tick_duration = sequencer.average_tick_duration(0);
        assert_eq!(DEFAULT_TICK_DURATION_US, tick_duration.to_micros());

        let tick_duration = sequencer.average_tick_duration(100);
        assert_eq!(100, tick_duration.to_micros());

        sequencer.average_tick_duration(200);
        sequencer.average_tick_duration(300);
        sequencer.average_tick_duration(350);
        sequencer.average_tick_duration(400);
        let tick_duration = sequencer.average_tick_duration(450);
        assert_eq!(75, tick_duration.to_micros());
    }
}
