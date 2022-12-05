#![cfg_attr(not(test), no_std)]

pub mod machines;
pub mod params;
pub mod sequencer;

extern crate alloc;

use alloc::boxed::Box;
use core::cmp::Ordering;
use heapless::Vec;
use midi_types::{Channel, Note, Value14, Value7};

use machines::{
    machine_from_id, unitmachine::UnitMachine, Machine, GROOVE_MACHINE_IDS, MELODY_MACHINE_IDS,
};
use params::{EnumParam, NumberParam, ParamList};

pub const TRACK_COUNT: usize = 16;

const TRACK_MIN_LENGTH: u8 = 1; // because live performance effect of repeating a single step
const TRACK_MAX_LENGTH: usize = 32;
const TRACK_DEFAULT_LENGTH: u8 = 8; // because techno

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

#[derive(Clone, Copy, Debug, Default)]
pub enum TimeDivision {
    ThirtySecond = 3,
    #[default]
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

/// Generate a sequence by piping the initial sequence through the set of configured machines.
fn generate_sequence(
    length: u8,
    groove_machine: &dyn Machine,
    melody_machine: &dyn Machine,
) -> Sequence {
    melody_machine.apply(groove_machine.apply(initial_sequence(length)))
}

fn initial_sequence(length: u8) -> Sequence {
    (0..length).map(|_i| Some(Step::new(60))).collect()
}

fn track_params() -> ParamList {
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
    params
}

#[derive(Debug)]
pub struct Track {
    pub time_division: TimeDivision,
    pub length: u8,
    pub midi_channel: Channel,
    pub sequence: Sequence,
    pub groove_machine: Box<dyn Machine>,
    pub melody_machine: Box<dyn Machine>,
    pub params: ParamList,
}

impl Track {
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
        self.generate_sequence();
    }

    pub fn generate_sequence(&mut self) {
        self.sequence = generate_sequence(self.length, &*self.groove_machine, &*self.melody_machine)
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
        self.sequence
            .get(self.step_num(tick) as usize)
            .unwrap()
            .as_ref()
    }
}

impl Default for Track {
    fn default() -> Track {
        let length = TRACK_DEFAULT_LENGTH;
        let groove_machine = UnitMachine::new();
        let melody_machine = UnitMachine::new();
        let sequence = generate_sequence(length, &groove_machine, &melody_machine);
        let params = track_params();
        Track {
            time_division: TimeDivision::Sixteenth,
            length,
            midi_channel: 0.into(),
            sequence,
            groove_machine: Box::new(groove_machine),
            melody_machine: Box::new(melody_machine),
            params,
        }
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

    #[test]
    fn track_default_generates_sequence_correctly() {
        let t = Track::default();
        let expected: Sequence = (0..8).map(|_i| Some(Step::new(60))).collect();
        assert_eq!(expected, t.sequence);
    }

    #[test]
    fn track_apply_params_generates_sequence_correctly() {
        let mut t = Track::default();
        t.params[1].increment(-1);
        t.apply_params();
        let expected: Sequence = (0..7).map(|_i| Some(Step::new(60))).collect();
        assert_eq!(expected, t.sequence);
    }
}
