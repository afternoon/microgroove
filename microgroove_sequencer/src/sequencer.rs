use alloc::boxed::Box;
use core::fmt::{Display, Formatter, Result as FmtResult};
use fugit::{ExtU64, MicrosDurationU64};
use heapless::{HistoryBuffer, Vec};
use midi_types::MidiMessage;

use crate::{
    param::{Param, ParamList, ParamValue},
    TimeDivision, Track, TRACK_COUNT,
};

// TODO will cause issues if polyphony
const MAX_MESSAGES_PER_TICK: usize = TRACK_COUNT * 2;

const MIDI_HISTORY_SAMPLE_COUNT: usize = 6;

#[derive(Debug)]
pub enum SequencerError {
    EnableTrackError(),
}

#[derive(Debug, PartialEq)]
pub enum ScheduledMidiMessage {
    Immediate(MidiMessage),
    Delayed(MidiMessage, MicrosDurationU64),
}

const DEFAULT_BPM: u64 = 130;
const DEFAULT_TICK_DURATION_US: u64 = (60_000_000 / DEFAULT_BPM) / 24;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Swing {
    #[default]
    None,
    Mpc54,
    Mpc58,
    Mpc62,
    Mpc66,
    Mpc70,
    Mpc75,
}

impl Swing {
    pub fn as_percentage(&self) -> u8 {
        match self {
            Swing::None => 50,
            Swing::Mpc54 => 54,
            Swing::Mpc58 => 58,
            Swing::Mpc62 => 62,
            Swing::Mpc66 => 66,
            Swing::Mpc70 => 70,
            Swing::Mpc75 => 75,
        }
    }
}

impl Display for Swing {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "{}", self.as_percentage())
    }
}

impl Into<u8> for Swing {
    fn into(self) -> u8 {
        self as u8
    }
}

impl TryFrom<u8> for Swing {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Swing::None),
            1 => Ok(Swing::Mpc54),
            2 => Ok(Swing::Mpc58),
            3 => Ok(Swing::Mpc62),
            4 => Ok(Swing::Mpc66),
            5 => Ok(Swing::Mpc70),
            6 => Ok(Swing::Mpc75),
            _ => Err(()),
        }
    }
}

pub struct Sequencer {
    pub tracks: Vec<Option<Track>, TRACK_COUNT>,
    tick: u32,
    playing: bool,
    params: ParamList,
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
            params: ParamList::from_slice(&[
                // if ordering changes, need to update getters and setters, e.g. swing/set_swing
                Box::new(Param::new_swing_param("SWING")),
            ])
            .unwrap(),
            last_tick_instant_us: None,
            midi_tick_history: HistoryBuffer::<u64, MIDI_HISTORY_SAMPLE_COUNT>::new(),
        }
    }
}

impl Sequencer {
    pub fn playing(&self) -> bool {
        self.playing
    }

    pub fn params(&self) -> &ParamList {
        &self.params
    }

    pub fn params_mut(&mut self) -> &mut ParamList {
        &mut self.params
    }

    pub fn tick(&self) -> u32 {
        self.tick
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

    pub fn swing(&self) -> Swing {
        self.params[0]
            .value()
            .try_into()
            .expect("invalid swing parameter for sequencer")
    }

    pub fn set_swing(&mut self, swing: Swing) {
        self.params[0].set(ParamValue::Swing(swing));
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

        let apply_swing = self.swing() != Swing::None && self.tick % 12 == 6;
        let swing_delay = (tick_duration * (self.swing().as_percentage() - 50) as u32) / 8;

        for track in &self.tracks {
            if let Some(track) = track {
                if let Some(step) = track.step_at_tick(self.tick) {
                    let note_on_message =
                        MidiMessage::NoteOn(track.midi_channel, step.note.into(), step.velocity);
                    if apply_swing {
                        output_messages
                            .push(ScheduledMidiMessage::Delayed(note_on_message, swing_delay))
                            .unwrap();
                    } else {
                        output_messages
                            .push(ScheduledMidiMessage::Immediate(note_on_message))
                            .unwrap();
                    }

                    let note_off_message =
                        MidiMessage::NoteOff(track.midi_channel, step.note.into(), 0.into());
                    let mut note_off_time = ((tick_duration.to_micros()
                        * (TimeDivision::division_length_24ppqn(track.time_division) as u64)
                        * step.length_step_cents as u64)
                        / 100)
                        .micros();
                    if apply_swing {
                        note_off_time += swing_delay;
                    }
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
        assert_eq!(false, sequencer.playing());
        assert_eq!(0, sequencer.tick);
        sequencer.start_playing();
        assert_eq!(true, sequencer.playing());

        sequencer.advance(1);
        sequencer.stop_playing();
        assert_eq!(false, sequencer.playing());

        sequencer.advance(1); // should be ignored because sequencer stopped
        sequencer.continue_playing();
        sequencer.advance(1);
        assert_eq!(true, sequencer.playing());
        assert_eq!(2, sequencer.tick);

        sequencer.stop_playing();
        assert_eq!(2, sequencer.tick);

        sequencer.start_playing();
        assert_eq!(true, sequencer.playing());
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

    #[test]
    fn sequencer_advance_should_output_immediate_note_on_and_delayed_note_off_messages() {
        let mut now_us = 0;
        let mut sequencer = Sequencer::default();
        let generator = SequenceGenerator::default();
        let mut machine_resources = MachineResources::new();
        let mut new_track = Track::default();
        new_track.sequence = generator.generate(new_track.length, &mut machine_resources);
        sequencer.enable_track(0, new_track);
        sequencer.start_playing();
        let mut output_messages = vec![];
        for _ in 0..48 {
            let step_messages = sequencer.advance(now_us);
            output_messages.extend(step_messages.into_iter());
            now_us += DEFAULT_TICK_DURATION_US;
        }
        assert_eq!(16, output_messages.len()); // 8 note on/note off pairs
        let expected_note_on =
            ScheduledMidiMessage::Immediate(MidiMessage::NoteOn(0.into(), 60.into(), 127.into()));
        let expected_note_off = ScheduledMidiMessage::Delayed(
            MidiMessage::NoteOff(0.into(), 60.into(), 0.into()),
            92304.micros(),
        );
        assert_eq!(expected_note_on, output_messages[0]);
        assert_eq!(expected_note_off, output_messages[1]);
        assert_eq!(expected_note_on, output_messages[2]);
        assert_eq!(expected_note_off, output_messages[3]);
        assert_eq!(expected_note_on, output_messages[4]);
        assert_eq!(expected_note_off, output_messages[5]);
        assert_eq!(expected_note_on, output_messages[6]);
        assert_eq!(expected_note_off, output_messages[7]);
        assert_eq!(expected_note_on, output_messages[8]);
        assert_eq!(expected_note_off, output_messages[9]);
        assert_eq!(expected_note_on, output_messages[10]);
        assert_eq!(expected_note_off, output_messages[11]);
        assert_eq!(expected_note_on, output_messages[12]);
        assert_eq!(expected_note_off, output_messages[13]);
        assert_eq!(expected_note_on, output_messages[14]);
        assert_eq!(expected_note_off, output_messages[15]);
    }

    #[test]
    fn sequencer_advance_with_swing_enabled_should_output_delayed_note_on_messages_for_swung_steps()
    {
        let mut now_us = 0;
        let mut sequencer = Sequencer::default();
        let generator = SequenceGenerator::default();
        let mut machine_resources = MachineResources::new();
        let mut new_track = Track::default();
        new_track.sequence = generator.generate(new_track.length, &mut machine_resources);
        sequencer.enable_track(0, new_track);
        sequencer.set_swing(Swing::Mpc54);
        sequencer.start_playing();
        let mut output_messages = vec![];
        for _ in 0..48 {
            let step_messages = sequencer.advance(now_us);
            output_messages.extend(step_messages.into_iter());
            now_us += DEFAULT_TICK_DURATION_US;
        }
        assert_eq!(16, output_messages.len()); // 8 note on/note off pairs
        let expected_note_on =
            ScheduledMidiMessage::Immediate(MidiMessage::NoteOn(0.into(), 60.into(), 127.into()));
        let expected_note_on_with_swing = ScheduledMidiMessage::Delayed(
            MidiMessage::NoteOn(0.into(), 60.into(), 127.into()),
            9615.micros(),
        );
        let expected_note_off = ScheduledMidiMessage::Delayed(
            MidiMessage::NoteOff(0.into(), 60.into(), 0.into()),
            92304.micros(),
        );
        let expected_note_off_with_swing = ScheduledMidiMessage::Delayed(
            MidiMessage::NoteOff(0.into(), 60.into(), 0.into()),
            (92304 + 9615).micros(),
        );
        assert_eq!(expected_note_on, output_messages[0]);
        assert_eq!(expected_note_off, output_messages[1]);
        assert_eq!(expected_note_on_with_swing, output_messages[2]);
        assert_eq!(expected_note_off_with_swing, output_messages[3]);
        assert_eq!(expected_note_on, output_messages[4]);
        assert_eq!(expected_note_off, output_messages[5]);
        assert_eq!(expected_note_on_with_swing, output_messages[6]);
        assert_eq!(expected_note_off_with_swing, output_messages[7]);
        assert_eq!(expected_note_on, output_messages[8]);
        assert_eq!(expected_note_off, output_messages[9]);
        assert_eq!(expected_note_on_with_swing, output_messages[10]);
        assert_eq!(expected_note_off_with_swing, output_messages[11]);
        assert_eq!(expected_note_on, output_messages[12]);
        assert_eq!(expected_note_off, output_messages[13]);
        assert_eq!(expected_note_on_with_swing, output_messages[14]);
        assert_eq!(expected_note_off_with_swing, output_messages[15]);
    }
}
