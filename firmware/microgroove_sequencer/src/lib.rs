#![cfg_attr(not(test), no_std)]

pub mod machine;
pub mod machine_resources;
pub mod midi;
pub mod param;
pub mod part;
pub mod quantizer;
pub mod sequence_generator;
pub mod sequencer;

extern crate alloc;

use midi::{Note, NoteError};
use param::{Param, ParamError, ParamList};
use sequence_generator::SequenceGenerator;

use alloc::boxed::Box;
use core::{
    cmp::Ordering,
    fmt::{Display, Formatter, Result as FmtResult},
    slice::{Iter, IterMut},
};
use heapless::Vec;
use midi_types::{Channel, Value14, Value7};

pub const TRACK_COUNT: usize = 8;

const TRACK_MIN_LENGTH: u8 = 1; // because live performance effect of repeating a single step
const TRACK_MAX_LENGTH: u8 = 32;
const TRACK_DEFAULT_LENGTH: u8 = 8; // because techno

const SEQUENCE_MAX_STEPS: usize = TRACK_MAX_LENGTH as usize;

const TRACK_MIN_NUM: u8 = 1;

const MIDI_MIN_CHANNEL: u8 = 1;
const MIDI_MAX_CHANNEL: u8 = 16;

pub fn map_to_range(x: i32, in_min: i32, in_max: i32, out_min: i32, out_max: i32) -> i32 {
    (x - in_min) * (out_max - out_min + 1) / (in_max - in_min + 1) + out_min
}

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
    pub fn new(note: u8) -> Result<Step, NoteError> {
        Ok(Step {
            note: note.try_into()?,
            velocity: 127.into(),
            pitch_bend: 0u16.into(),
            length_step_cents: 80,
            delay: 0,
        })
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
    ThirtySecond,
    #[default]
    Sixteenth,
    Eigth,
    Quarter,
    Whole,
}

impl TimeDivision {
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

    pub fn division_length_24ppqn(time_div: TimeDivision) -> u8 {
        match time_div {
            TimeDivision::ThirtySecond => 3,
            TimeDivision::Sixteenth => 6,
            TimeDivision::Eigth => 12,
            TimeDivision::Quarter => 24,
            TimeDivision::Whole => 96,
        }
    }
}

impl Display for TimeDivision {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
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

impl TryFrom<u8> for TimeDivision {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(TimeDivision::ThirtySecond),
            1 => Ok(TimeDivision::Sixteenth),
            2 => Ok(TimeDivision::Eigth),
            3 => Ok(TimeDivision::Quarter),
            4 => Ok(TimeDivision::Whole),
            _ => Err(()),
        }
    }
}

type StepVec = Vec<Option<Step>, SEQUENCE_MAX_STEPS>;

#[derive(Clone, Debug)]
pub struct Sequence {
    pub steps: StepVec,
}

impl Sequence {
    pub fn new(steps: StepVec) -> Sequence {
        Sequence { steps }
    }

    pub fn len(&self) -> usize {
        self.steps.len()
    }
    pub fn iter(&self) -> Iter<Option<Step>> {
        self.steps.iter()
    }
    pub fn iter_mut(&mut self) -> IterMut<Option<Step>> {
        self.steps.iter_mut()
    }
    pub fn as_slice(&self) -> &[Option<Step>] {
        self.steps.as_slice()
    }

    pub fn set_steps(mut self, steps: Vec<Option<Step>, SEQUENCE_MAX_STEPS>) -> Self {
        self.steps = steps;
        self
    }

    pub fn rotate_left(mut self, amount: usize) -> Sequence {
        self.steps.rotate_left(amount);
        self
    }

    pub fn rotate_right(mut self, amount: usize) -> Sequence {
        self.steps.rotate_right(amount);
        self
    }

    pub fn map_notes(mut self, mut f: impl FnMut(Note) -> Note) -> Self {
        for step in self.steps.iter_mut() {
            if let Some(step) = step {
                step.note = f(step.note);
            }
        }
        self
    }

    pub fn set_notes<I>(mut self, notes: I) -> Self
    where
        I: IntoIterator<Item = Note>,
    {
        let mut notes = notes.into_iter();
        for step in self.steps.iter_mut() {
            let next_note = notes.next();
            if let Some(step) = step {
                step.note = next_note.expect("should get next note");
            }
        }
        self
    }

    pub fn mask_steps<I>(mut self, step_mask: I) -> Self
    where
        I: IntoIterator<Item = bool>,
    {
        let steps_unmasked_pairs = self.steps.iter_mut().zip(step_mask);
        for (step, unmasked) in steps_unmasked_pairs {
            if !unmasked {
                step.take();
            }
        }
        self
    }
}

impl PartialEq for Sequence {
    fn eq(&self, other: &Self) -> bool {
        self.steps == other.steps
    }
}

impl FromIterator<Option<Step>> for Sequence {
    fn from_iter<T>(steps: T) -> Self
    where
        T: IntoIterator<Item = Option<Step>>,
    {
        Sequence::new(StepVec::from_iter(steps))
    }
}

#[derive(Debug)]
pub struct Track {
    pub time_division: TimeDivision,
    pub length: u8,
    pub midi_channel: Channel,
    pub sequence: Sequence,
    pub params: ParamList,
}

impl Default for Track {
    fn default() -> Track {
        let length = TRACK_DEFAULT_LENGTH;
        let sequence = SequenceGenerator::initial_sequence(length);
        let params = Track::param_defintions();
        Track {
            time_division: Default::default(),
            length,
            midi_channel: 0.into(),
            sequence,
            params,
        }
    }
}

impl Track {
    fn param_defintions() -> ParamList {
        ParamList::from_slice(&[
            Box::new(Param::new_rhythm_machine_id_param("RHYTHM")),
            Box::new(Param::new_number_param(
                "LEN",
                TRACK_MIN_LENGTH,
                TRACK_MAX_LENGTH,
                TRACK_DEFAULT_LENGTH,
            )),
            Box::new(Param::new_number_param(
                "TRACK",
                TRACK_MIN_NUM,
                TRACK_COUNT as u8,
                TRACK_MIN_NUM,
            )),
            Box::new(Param::new_melody_machine_id_param("MELODY")),
            Box::new(Param::new_time_division_param("SPD")),
            Box::new(Param::new_number_param(
                "CHAN",
                MIDI_MIN_CHANNEL,
                MIDI_MAX_CHANNEL,
                MIDI_MIN_CHANNEL,
            )),
        ])
        .expect("should create track param list from slice")
    }

    pub fn params(&self) -> &ParamList {
        &self.params
    }

    pub fn params_mut(&mut self) -> &mut ParamList {
        &mut self.params
    }

    pub fn apply_params(&mut self) -> Result<(), ParamError> {
        // params 0 (rhythm machine), 2 (track number) and 3 (melody machine) are intentionally ignored
        // they are "virtual parameters" which don't actually relate to a `Track` at all. They're
        // handled by microgroove_app::input::map_encoder_values directly.
        self.length = self.params[1].value().try_into()?;
        self.time_division = self.params[4].value().try_into()?;
        let channel_num: u8 = self.params[5].value().try_into()?;
        self.midi_channel = channel_num.into();
        Ok(())
    }

    pub fn should_play_on_tick(&self, tick: u32) -> bool {
        tick % (TimeDivision::division_length_24ppqn(self.time_division) as u32) == 0
    }

    pub fn step_num(&self, tick: u32) -> u8 {
        (tick / (TimeDivision::division_length_24ppqn(self.time_division) as u32)
            % self.length as u32) as u8
    }

    pub fn step_at_tick(&self, tick: u32) -> Option<&Step> {
        if !self.should_play_on_tick(tick) {
            return None;
        }
        self.sequence
            .steps
            .get(self.step_num(tick) as usize)
            .expect("should get step at tick")
            .as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_to_range_maps_to_range() {
        assert_eq!(10, map_to_range(100, 0, 100, 0, 10));
        assert_eq!(5, map_to_range(50, 0, 100, 0, 10));
        assert_eq!(0, map_to_range(0, 0, 100, 0, 10));
        assert_eq!(1, map_to_range(10, 0, 100, 0, 10));
        assert_eq!(66, map_to_range(63, 0, 127, 60, 72));
    }

    #[test]
    fn steps_are_correctly_ordered() {
        let (s1, s2) = (Step::new(60).unwrap(), Step::new(61).unwrap());
        assert!(s1 < s2);
    }

    #[test]
    fn track_default_generates_sequence_correctly() {
        let t = Track::default();
        let expected: Sequence = Sequence::new((0..8).map(|_i| Step::new(60).ok()).collect());
        assert_eq!(expected, t.sequence);
    }

    #[test]
    fn sequence_set_notes_should_set_note_values_from_intoiterator() {
        let seq = SequenceGenerator::initial_sequence(8);
        let notes: [Note; 8] = [60, 61, 62, 63, 64, 65, 66, 67].map(|i| i.try_into().unwrap());
        let seq = seq.set_notes(notes);
        let result: Vec<Note, 8> = seq.iter().map(|step| step.as_ref().unwrap().note).collect();
        assert_eq!(notes, result);
    }
}
