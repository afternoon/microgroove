#![cfg_attr(not(test), no_std)]

pub mod machines;
pub mod params;
pub mod sequencer;

extern crate alloc;

use alloc::boxed::Box;
use core::cmp::Ordering;
use heapless::Vec;
use midi_types::{Channel, Note, Value14, Value7};

use machines::{machine_from_id, Machine, GROOVE_MACHINE_IDS, MELODY_MACHINE_IDS};
use params::{EnumParam, NumberParam, ParamList};

pub const TRACK_COUNT: usize = 16;

const TRACK_MIN_LENGTH: usize = 1; // because live performance effect of repeating a single step
const TRACK_MAX_LENGTH: usize = 32;
const TRACK_DEFAULT_LENGTH: usize = 8; // because techno

const TRACK_MIN_NUM: i8 = 1;
const TRACK_DEFAULT_NUM: i8 = 1;

const MIDI_MIN_CHANNEL: i8 = 1;
const MIDI_MAX_CHANNEL: i8 = 16;
const MIDI_DEFAULT_CHANNEL: i8 = 1;

/// Represent a step in a musical sequence.
#[derive(Clone, Debug)]
pub struct Step {
    pub note: Note,
    pub velocity: Value7,
    pub pitch_bend: Value14,

    /// Note gate time as % of step time, e.g. 80 = 80%. Step time is defined by
    /// Track::time_division.
    pub length_step_cents: u8,

    /// Delay playing this step for % of track time division. Used for swing. Can be abused
    /// for general timing madness. Note that its not possible to play a step early. This
    /// is because Microgroove depends on an external clock.
    pub delay: u8,
}

impl Step {
    pub fn new(note: u8) -> Step {
        Step {
            note: note.into(),
            velocity: 127.into(),
            pitch_bend: 0u16.into(),
            length_step_cents: 80,
            delay: 0,
        }
    }
}

impl PartialEq for Step {
    fn eq(&self, other: &Self) -> bool {
        let self_note_num: u8 = self.note.into();
        let other_note_num: u8 = other.note.into();
        self_note_num == other_note_num
    }
}

impl Eq for Step {}

impl PartialOrd for Step {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Step {
    fn cmp(&self, other: &Self) -> Ordering {
        let self_note_num: u8 = self.note.into();
        let other_note_num: u8 = other.note.into();
        self_note_num.cmp(&other_note_num)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum TimeDivision {
    ThirtySecond = 3,
    Sixteenth = 6,
    Eigth = 12,
    Quarter = 24,
    Whole = 96,
}

pub fn time_division_from_id(id: &str) -> TimeDivision {
    match id {
        "1/32" => TimeDivision::ThirtySecond,
        "1/16" => TimeDivision::Sixteenth,
        "1/8" => TimeDivision::Eigth,
        "1/4" => TimeDivision::Quarter,
        "1" => TimeDivision::Whole,
        _ => TimeDivision::Sixteenth,
    }
}

pub type Sequence = Vec<Option<Step>, TRACK_MAX_LENGTH>;

pub trait SequenceProcessor {
    fn apply(&self, sequence: Sequence) -> Sequence;
}

#[derive(Debug)]
pub struct Track {
    pub time_division: TimeDivision,
    pub length: u8,
    pub midi_channel: Channel,
    pub steps: Sequence,
    pub groove_machine: Box<dyn Machine>,
    pub melody_machine: Box<dyn Machine>,
    params: ParamList,
}

impl Track {
    pub fn new(
        groove_machine: impl Machine + 'static,
        melody_machine: impl Machine + 'static,
    ) -> Track {
        let mut params: ParamList = Vec::new();
        params
            .push(Box::new(EnumParam::new("GROOVE", GROOVE_MACHINE_IDS)))
            .unwrap();
        params
            .push(Box::new(NumberParam::new(
                "LEN",
                TRACK_MIN_LENGTH as i8,
                TRACK_MAX_LENGTH as i8,
                TRACK_DEFAULT_LENGTH as i8,
            )))
            .unwrap();
        params
            .push(Box::new(NumberParam::new(
                "TRACK",
                TRACK_MIN_NUM,
                TRACK_COUNT as i8,
                TRACK_DEFAULT_NUM,
            )))
            .unwrap();
        params
            .push(Box::new(EnumParam::new("MELODY", MELODY_MACHINE_IDS)))
            .unwrap();
        params
            .push(Box::new(EnumParam::new("SPD", "1/32 1/16 1/8 1/4 1")))
            .unwrap();
        params
            .push(Box::new(NumberParam::new(
                "CHAN",
                MIDI_MIN_CHANNEL,
                MIDI_MAX_CHANNEL,
                MIDI_DEFAULT_CHANNEL,
            )))
            .unwrap();
        Track {
            time_division: TimeDivision::Sixteenth,
            length: 16,
            midi_channel: 0.into(),
            steps: Track::generate_sequence(),
            groove_machine: Box::new(groove_machine),
            melody_machine: Box::new(melody_machine),
            params,
        }
    }

    pub fn params(&self) -> &ParamList {
        &self.params
    }

    pub fn params_mut(&mut self) -> &mut ParamList {
        &mut self.params
    }

    pub fn apply_params(&mut self) {
        self.groove_machine =
            Box::new(machine_from_id(self.params[0].value_str().as_str()).unwrap());
        self.length = self.params[1].value_i8().unwrap() as u8;
        // params[2], track number, is intentionally ignored, its handled by Sequencer::set_current_track
        self.melody_machine =
            Box::new(machine_from_id(self.params[3].value_str().as_str()).unwrap());
        self.time_division = time_division_from_id(self.params[4].value_str().as_str());
        self.midi_channel = (self.params[5].value_i8().unwrap() as u8).into();
    }

    fn generate_sequence() -> Sequence {
        Self::initial_sequence()
        // TODO pipe sequence through machines
    }

    fn initial_sequence() -> Sequence {
        (0..16).map(|_i| Some(Step::new(60))).collect()
    }

    pub fn should_play_on_tick(&self, tick: u32) -> bool {
        tick % (self.time_division as u32) == 0
    }

    pub fn step_num(&self, tick: u32) -> u32 {
        tick / (self.time_division as u32) % self.length as u32
    }

    pub fn step_at_tick(&self, tick: u32) -> Option<&Step> {
        if !self.should_play_on_tick(tick) {
            return None;
        }
        self.steps
            .get(self.step_num(tick) as usize)
            .unwrap()
            .as_ref()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn steps_are_correctly_ordered() {
        let (s1, s2) = (Step::new(60), Step::new(61));
        assert!(s1 < s2);
    }
}
