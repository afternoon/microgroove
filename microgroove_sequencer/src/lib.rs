#![cfg_attr(not(test), no_std)]

pub mod machine;
pub mod midi;
pub mod param;
pub mod sequencer;

extern crate alloc;

use alloc::boxed::Box;
use core::{
    cmp::Ordering,
    fmt::{Display, Formatter, Result},
};
use heapless::Vec;
use midi_types::{Channel, Note, Value14, Value7};

use machine::{groove_machine_ids, machine_from_id, unit_machine::UnitMachine, Machine, melody_machine_ids, MachineResources};
use param::{Param, ParamList, ParamValue};

pub const TRACK_COUNT: usize = 16;

const TRACK_MIN_LENGTH: u8 = 1; // because live performance effect of repeating a single step
const TRACK_MAX_LENGTH: u8 = 32;
const TRACK_DEFAULT_LENGTH: u8 = 8; // because techno

const SEQUENCE_MAX_STEPS: usize = TRACK_MAX_LENGTH as usize;

const TRACK_MIN_NUM: u8 = 1;

const MIDI_MIN_CHANNEL: u8 = 1;
const MIDI_MAX_CHANNEL: u8 = 16;

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

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum TimeDivision {
    ThirtySecond = 3,
    #[default]
    Sixteenth = 6,
    Eigth = 12,
    Quarter = 24,
    Whole = 96,
}

impl TimeDivision {
    pub fn all_variants() -> Vec<TimeDivision, 5> {
        Vec::from_slice(&[
            TimeDivision::ThirtySecond,
            TimeDivision::Sixteenth,
            TimeDivision::Eigth,
            TimeDivision::Quarter,
            TimeDivision::Whole,
        ])
        .unwrap()
    }

    // TODO TryFrom
    pub fn from_id(id: &str) -> TimeDivision {
        match id {
            "1/32" => TimeDivision::ThirtySecond,
            "1/16" => TimeDivision::Sixteenth,
            "1/8" => TimeDivision::Eigth,
            "1/4" => TimeDivision::Quarter,
            "1" => TimeDivision::Whole,
            _ => TimeDivision::Sixteenth,
        }
    }
}

impl Display for TimeDivision {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(
            f,
            "{}",
            // TODO impl Into<String> for TimeDivision
            match *self {
                TimeDivision::ThirtySecond => "1/32",
                TimeDivision::Sixteenth => "1/16",
                TimeDivision::Eigth => "1/8",
                TimeDivision::Quarter => "1/4",
                TimeDivision::Whole => "1",
            }
        )
    }
}

pub type Sequence = Vec<Option<Step>, SEQUENCE_MAX_STEPS>;

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
        .push(Box::new(
            Param::new(
                "GROOVE".into(),
                ParamValue::GrooveMachine("UNIT".into()),
                groove_machine_ids()
                    .iter()
                    .map(|id| ParamValue::GrooveMachine(id.clone()))
                    .collect(),
            )
            .unwrap(),
        ))
        .unwrap();
    params
        .push(Box::new(
            Param::new(
                "LEN".into(),
                ParamValue::Number(TRACK_DEFAULT_LENGTH),
                (TRACK_MIN_LENGTH..=TRACK_MAX_LENGTH)
                    .map(ParamValue::Number)
                    .collect(),
            )
            .unwrap(),
        ))
        .unwrap();
    params
        .push(Box::new(
            Param::new(
                "TRACK".into(),
                ParamValue::Number(TRACK_MIN_NUM),
                (TRACK_MIN_NUM..=TRACK_COUNT as u8)
                    .map(ParamValue::Number)
                    .collect(),
            )
            .unwrap(),
        ))
        .unwrap();
    params
        .push(Box::new(
            Param::new(
                "MELODY".into(),
                ParamValue::MelodyMachine("UNIT".into()),
                melody_machine_ids()
                    .iter()
                    .map(|id| ParamValue::MelodyMachine(id.clone()))
                    .collect(),
            )
            .unwrap(),
        ))
        .unwrap();
    params
        .push(Box::new(
            Param::new(
                "SPD".into(),
                ParamValue::TimeDivision(TimeDivision::Sixteenth),
                TimeDivision::all_variants()
                    .iter()
                    .map(|&time_div| ParamValue::TimeDivision(time_div))
                    .collect(),
            )
            .unwrap(),
        ))
        .unwrap();
    params
        .push(Box::new(
            Param::new(
                "CHAN".into(),
                ParamValue::Number(MIDI_MIN_CHANNEL),
                (MIDI_MIN_CHANNEL..=MIDI_MAX_CHANNEL)
                    .map(ParamValue::Number)
                    .collect(),
            )
            .unwrap(),
        ))
        .unwrap();
    params
}

#[derive(Debug)]
pub struct Track<'a> {
    machine_resources: &'a dyn MachineResources,
    pub time_division: TimeDivision,
    pub length: u8,
    pub midi_channel: Channel,
    pub sequence: Sequence,
    pub groove_machine: Box<dyn Machine + 'a>,
    pub melody_machine: Box<dyn Machine + 'a>,
    pub params: ParamList,
}

impl<'a> Track<'a> {
    fn new(machine_resources: &'a dyn MachineResources) -> Track<'a> {
        let length = TRACK_DEFAULT_LENGTH;
        let groove_machine = UnitMachine::new(machine_resources);
        let melody_machine = UnitMachine::new(machine_resources);
        let sequence = generate_sequence(length, &groove_machine, &melody_machine);
        let params = track_params();
        Track {
            machine_resources,
            time_division: Default::default(),
            length,
            midi_channel: 0.into(),
            sequence,
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
        match self.params[0].value() {
            ParamValue::GrooveMachine(machine_id) => {
                self.groove_machine = machine_from_id(machine_id.as_str(), self.machine_resources).unwrap();
            }
            unexpected => panic!("unexpected track param[0]: {:?}", unexpected)
        };
        match self.params[1].value() {
            ParamValue::Number(length) => {
                self.length = length;
            }
            unexpected => panic!("unexpected track param[1]: {:?}", unexpected)
        };

        // params[2], track number, is intentionally ignored, its handled by Sequencer::set_current_track

        match self.params[3].value() {
            ParamValue::MelodyMachine(machine_id) => {
                self.melody_machine = machine_from_id(machine_id.as_str(), self.machine_resources).unwrap();
            }
            unexpected => panic!("unexpected track param[3]: {:?}", unexpected)
        }
        match self.params[4].value() {
            ParamValue::TimeDivision(time_division) => {
                self.time_division = time_division;
            }
            unexpected => panic!("unexpected track param[4]: {:?}", unexpected)
        }
        match self.params[5].value() {
            ParamValue::Number(midi_channel) => {
                self.midi_channel = midi_channel.into();
            }
            unexpected => panic!("unexpected track param[5]: {:?}", unexpected)
        };
        self.generate_sequence();
    }

    pub fn generate_sequence(&mut self) {
        self.sequence = generate_sequence(self.length, &*self.groove_machine, &*self.melody_machine)
    }

    pub fn should_play_on_tick(&self, tick: u32) -> bool {
        tick % (self.time_division as u32) == 0
    }

    pub fn step_num(&self, tick: u32) -> u8 {
        (tick / (self.time_division as u32) % self.length as u32) as u8
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

#[cfg(test)]
mod test {
    use crate::machine::tests::TestMachineResources;

    use super::*;

    #[test]
    fn steps_are_correctly_ordered() {
        let (s1, s2) = (Step::new(60), Step::new(61));
        assert!(s1 < s2);
    }

    #[test]
    fn track_default_generates_sequence_correctly() {
        let machine_resources = TestMachineResources::default();
        let t = Track::new(&machine_resources);
        let expected: Sequence = (0..8).map(|_i| Some(Step::new(60))).collect();
        assert_eq!(expected, t.sequence);
    }

    #[test]
    fn track_apply_params_generates_sequence_correctly() {
        let machine_resources = TestMachineResources::default();
        let mut t = Track::new(&machine_resources);
        t.params[1].increment(-1); // track length
        t.apply_params();
        assert_eq!(7, t.sequence.len());
    }
}
