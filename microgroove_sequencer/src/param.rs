/// Model parameters as mutable values with metadata (name).
use alloc::boxed::Box;
use core::cmp::PartialEq;
use core::fmt::{Debug, Display, Formatter, Result as FmtResult};
use heapless::{String, Vec};

use crate::sequencer::Swing;
use crate::{
    machine::{MelodyMachineId, RhythmMachineId},
    midi::Note,
    quantizer::{Key, Scale},
    TimeDivision,
};

pub fn wrapping_add(a: i32, b: i32, max: i32) -> i32 {
    let size = max + 1;
    ((a + b % size) + size) % size
}

// TODO now we have TryFrom/Into implementations for all types that are used as param values (u8,
// some enum), we don't need to tag the value twice, we can just manipulate params as u8s and call
// try_from or into where necessary. This is more ergonomic in both the implementation and as an
// API, because it removes the need for a match block.
//
// E.g.
//
//     let time_div = match param.value() {
//         ParamValue::TimeDivision(time_div) => time_div,
//         unexpected => panic!("unexpected param value"),
//     };
//
// Becomes:
//
//     let time_div: TimeDivision = param.value().try_into().unwrap();
//
// Which is nice.
//
// Refactoring is a medium task. Code here needs to change + ~6 call sites.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ParamValue {
    Number(u8),
    TimeDivision(TimeDivision),
    RhythmMachineId(RhythmMachineId),
    MelodyMachineId(MelodyMachineId),
    Note(Note),
    Scale(Scale),
    Key(Key),
    Swing(Swing),
}

impl Display for ParamValue {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            ParamValue::Number(num) => Display::fmt(&num, f),
            ParamValue::TimeDivision(time_div) => Display::fmt(&time_div, f),
            ParamValue::RhythmMachineId(id) => Display::fmt(&id, f),
            ParamValue::MelodyMachineId(id) => Display::fmt(&id, f),
            ParamValue::Note(note) => Display::fmt(&note, f),
            ParamValue::Scale(scale) => Display::fmt(&scale, f),
            ParamValue::Key(key) => Display::fmt(&key, f),
            ParamValue::Swing(swing) => Display::fmt(&swing, f),
        }
    }
}

impl From<ParamValue> for i32 {
    fn from(value: ParamValue) -> i32 {
        match value {
            ParamValue::Number(num) => num as i32,
            ParamValue::TimeDivision(time_div) => time_div as i32,
            ParamValue::RhythmMachineId(id) => id as i32,
            ParamValue::MelodyMachineId(id) => id as i32,
            ParamValue::Note(note) => note as i32,
            ParamValue::Scale(scale) => scale as i32,
            ParamValue::Key(key) => key as i32,
            ParamValue::Swing(swing) => swing as i32,
        }
    }
}

type ParamName = String<6>;

#[derive(Debug)]
pub enum ParamError {
    ValueError,
}

#[derive(Clone, Debug)]
pub struct Param {
    name: ParamName,
    value: ParamValue,
    min: ParamValue,
    max: ParamValue,
}

impl Param {
    pub fn new_number_param(name: &str, min: u8, max: u8, default: u8) -> Param {
        if default < min || default > max {
            panic!("param default out of bounds");
        }
        Param {
            name: name.into(),
            value: ParamValue::Number(default),
            min: ParamValue::Number(min),
            max: ParamValue::Number(max),
        }
    }

    pub fn new_time_division_param(name: &str) -> Param {
        Param {
            name: name.into(),
            value: ParamValue::TimeDivision(TimeDivision::default()),
            min: ParamValue::TimeDivision(TimeDivision::ThirtySecond),
            max: ParamValue::TimeDivision(TimeDivision::Whole),
        }
    }

    pub fn new_rhythm_machine_id_param(name: &str) -> Param {
        Param {
            name: name.into(),
            value: ParamValue::RhythmMachineId(RhythmMachineId::default()),
            min: ParamValue::RhythmMachineId(RhythmMachineId::Unit),
            max: ParamValue::RhythmMachineId(RhythmMachineId::Euclid),
        }
    }

    pub fn new_melody_machine_id_param(name: &str) -> Param {
        Param {
            name: name.into(),
            value: ParamValue::MelodyMachineId(MelodyMachineId::default()),
            min: ParamValue::MelodyMachineId(MelodyMachineId::Unit),
            max: ParamValue::MelodyMachineId(MelodyMachineId::Rand),
        }
    }

    pub fn new_note_param(name: &str) -> Param {
        Param {
            name: name.into(),
            value: ParamValue::Note(Note::default()),
            min: ParamValue::Note(Note::CMinus2),
            max: ParamValue::Note(Note::G8),
        }
    }

    pub fn new_scale_param(name: &str) -> Param {
        Param {
            name: name.into(),
            value: ParamValue::Scale(Scale::default()),
            min: ParamValue::Scale(Scale::Chromatic),
            max: ParamValue::Scale(Scale::OctaveAndFifth),
        }
    }

    pub fn new_key_param(name: &str) -> Param {
        Param {
            name: name.into(),
            value: ParamValue::Key(Key::default()),
            min: ParamValue::Key(Key::C),
            max: ParamValue::Key(Key::B),
        }
    }

    pub fn new_swing_param(name: &str) -> Param {
        Param {
            name: name.into(),
            value: ParamValue::Swing(Swing::default()),
            min: ParamValue::Swing(Swing::None),
            max: ParamValue::Swing(Swing::Mpc75),
        }
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn value(&self) -> ParamValue {
        self.value.clone()
    }

    pub fn set(&mut self, new_value: ParamValue) {
        // panic!("unexpected ParamValue variant");
        // if new_value < self.min || new_value > self.max {
        //     panic!("param default out of bounds");
        // }
        self.value = new_value;
    }

    pub fn set_from_u8(&mut self, new_value: u8) -> Result<(), ParamError> {
        match self.value {
            ParamValue::Number(_) => self.value = ParamValue::Number(new_value),
            ParamValue::TimeDivision(_) => new_value
                .try_into()
                .map(|val| self.value = ParamValue::TimeDivision(val))
                .map_err(|_| ParamError::ValueError)?,
            ParamValue::RhythmMachineId(_) => new_value
                .try_into()
                .map(|val| self.value = ParamValue::RhythmMachineId(val))
                .map_err(|_| ParamError::ValueError)?,
            ParamValue::MelodyMachineId(_) => new_value
                .try_into()
                .map(|val| self.value = ParamValue::MelodyMachineId(val))
                .map_err(|_| ParamError::ValueError)?,
            ParamValue::Note(_) => new_value
                .try_into()
                .map(|val| self.value = ParamValue::Note(val))
                .map_err(|_| ParamError::ValueError)?,
            ParamValue::Scale(_) => new_value
                .try_into()
                .map(|val| self.value = ParamValue::Scale(val))
                .map_err(|_| ParamError::ValueError)?,
            ParamValue::Key(_) => new_value
                .try_into()
                .map(|val| self.value = ParamValue::Key(val))
                .map_err(|_| ParamError::ValueError)?,
            ParamValue::Swing(_) => new_value
                .try_into()
                .map(|val| self.value = ParamValue::Swing(val))
                .map_err(|_| ParamError::ValueError)?,
        };
        Ok(())
    }

    pub fn increment(&mut self, n: i32) -> Result<(), ParamError> {
        let value_i32: i32 = self.value.into();
        let min_i32: i32 = self.min.into();
        let max_i32: i32 = self.max.into();
        let new_value = (wrapping_add(value_i32 - min_i32, n, max_i32 - min_i32) + min_i32) as u8;
        self.set_from_u8(new_value)
    }
}

pub type ParamList = Vec<Box<Param>, 6>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn param_cant_have_out_of_bounds_default() {
        let _ = Param::new_number_param("NUM", 1, 10, 0);
    }

    #[test]
    fn param_number_should_increment() {
        let mut param_number = Param::new_number_param("NUM", 0, 10, 0);
        param_number.increment(1).unwrap();
        match param_number.value() {
            ParamValue::Number(i) => assert_eq!(1, i),
            _ => panic!("unexpected param value"),
        }
    }

    #[test]
    fn param_number_starting_at_1_should_increment() {
        let mut param_number = Param::new_number_param("NUM", 1, 10, 1);
        param_number.increment(1).unwrap();
        match param_number.value() {
            ParamValue::Number(i) => assert_eq!(2, i),
            _ => panic!("unexpected param value"),
        }
        param_number.increment(10).unwrap();
        match param_number.value() {
            ParamValue::Number(i) => assert_eq!(2, i),
            _ => panic!("unexpected param value"),
        }
        param_number.increment(-5).unwrap();
        match param_number.value() {
            ParamValue::Number(i) => assert_eq!(7, i),
            _ => panic!("unexpected param value"),
        }
    }

    #[test]
    fn param_time_division_should_increment() {
        let mut param_time_div = Param::new_time_division_param("SPD");
        param_time_div.increment(1).unwrap();
        assert_param_time_division(TimeDivision::Eigth, &param_time_div);
        param_time_div.increment(9).unwrap();
        assert_param_time_division(TimeDivision::Sixteenth, &param_time_div);
        param_time_div.increment(-1).unwrap();
        assert_param_time_division(TimeDivision::ThirtySecond, &param_time_div);
        param_time_div.increment(-11).unwrap();
        assert_param_time_division(TimeDivision::Whole, &param_time_div);
    }

    #[test]
    fn param_enum_value_should_have_to_string() {
        let param_time_div = Param::new_time_division_param("SPD");
        match param_time_div.value() {
            ParamValue::TimeDivision(time_division) => {
                assert_eq!("1/16", time_division.to_string())
            }
            _ => panic!("unexpected param value"),
        }
    }

    #[test]
    fn param_list_can_store_different_param_types() {
        let param_number = Param::new_number_param("NUM", 0, 10, 0);
        let param_time_div = Param::new_time_division_param("SPD");
        let _param_list =
            ParamList::from_slice(&[Box::new(param_number), Box::new(param_time_div)]);
    }

    #[test]
    fn param_value_can_be_set() {
        let mut param_number = Param::new_number_param("NUM", 0, 10, 0);
        param_number.set(ParamValue::Number(1));
        match param_number.value() {
            ParamValue::Number(i) => assert_eq!(1, i),
            _ => panic!("unexpected param value"),
        }
    }

    #[test]
    #[should_panic]
    #[ignore = "unimplemented"]
    fn param_value_cant_be_set_to_value_out_of_range() {
        let mut param_number = Param::new_number_param("NUM", 0, 10, 0);
        param_number.set(ParamValue::Number(11));
    }

    #[test]
    #[should_panic]
    #[ignore = "unimplemented"]
    fn param_value_cant_be_set_to_different_paramvalue_variant() {
        let mut param_number = Param::new_number_param("NUM", 0, 10, 0);
        param_number.set(ParamValue::TimeDivision(TimeDivision::Sixteenth));
    }

    fn assert_param_time_division(expected: TimeDivision, param: &Param) {
        match param.value() {
            ParamValue::TimeDivision(time_div) => assert_eq!(expected, time_div),
            other => panic!("unexpected param value {:?}", other),
        }
    }
}
