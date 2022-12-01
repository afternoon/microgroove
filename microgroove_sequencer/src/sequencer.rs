extern crate alloc;

use embedded_midi::MidiMessage;
use fugit::{ExtU64, MicrosDurationU64};
use heapless::{HistoryBuffer, Vec};

use crate::{Track, TRACK_COUNT, machines::unitmachine::UnitMachine};

// TODO will cause issues if polyphony
const MAX_MESSAGES_PER_TICK: usize = TRACK_COUNT * 2;

const MIDI_HISTORY_SAMPLE_COUNT: usize = 6;

#[derive(Debug)]
pub enum ScheduledMidiMessage {
    Immediate(MidiMessage),
    Delayed(MidiMessage, MicrosDurationU64),
}

const DEFAULT_BPM: u64 = 130;
const DEFAULT_TICK_DURATION_US: u64 = (60 / DEFAULT_BPM) / 24;

pub fn new_track_with_default_machines() -> Track {
    Track::new(UnitMachine::new(), UnitMachine::new())
}

pub struct Sequencer {
    pub tracks: Vec<Option<Track>, TRACK_COUNT>,
    current_track: usize,
    playing: bool,
    tick: u32,
    last_tick_instant_us: Option<u64>,
    midi_tick_history: HistoryBuffer<u64, MIDI_HISTORY_SAMPLE_COUNT>,
}

impl Sequencer {
    pub fn new() -> Sequencer {
        // create a set of empty tracks
        let mut tracks = Vec::new();
        tracks
            .push(Some(new_track_with_default_machines()))
            .expect("inserting track into tracks vector should succeed");
        for _ in 1..TRACK_COUNT {
            tracks
                .push(None)
                .expect("inserting track into tracks vector should succeed");
        }
        Sequencer {
            tracks,
            current_track: 0,
            playing: false,
            tick: 0,
            last_tick_instant_us: None,
            midi_tick_history: HistoryBuffer::<u64, MIDI_HISTORY_SAMPLE_COUNT>::new(),
        }
    }

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

    pub fn current_track(&self) -> &Option<Track> {
        &self.tracks.get(self.current_track).unwrap()
    }

    pub fn current_track_mut(&mut self) -> &mut Option<Track> {
        self.tracks.get_mut(self.current_track).unwrap()
    }

    pub fn current_track_active_step_num(&self) -> Option<u32> {
        self.current_track()
            .as_ref()
            .map(|track| track.step_num(self.tick))
    }

    pub fn set_current_track(&mut self, new_track_num: u8) {
        self.current_track = new_track_num as usize;
    }

    pub fn advance(
        &mut self,
        now_us: u64,
    ) -> Vec<ScheduledMidiMessage, MAX_MESSAGES_PER_TICK> {
        let mut output_messages = Vec::new();
        let tick_duration = self.average_tick_duration(now_us);

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
                        * (track.time_division as u64)
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
            let last_tick_duration = last_tick_instant_us - now_us;
            self.midi_tick_history.write(last_tick_duration);
            tick_duration = (self.midi_tick_history.as_slice().iter().sum::<u64>()
                / self.midi_tick_history.len() as u64)
                .micros();
        }

        self.last_tick_instant_us = Some(now_us);

        tick_duration
    }
}
