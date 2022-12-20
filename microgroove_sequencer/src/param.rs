/// Model parameters as mutable values with metadata (name).
use alloc::boxed::Box;
use core::cmp::PartialEq;
use core::fmt::{Debug, Display, Formatter, Result as FmtResult};
use heapless::{String, Vec};

use crate::{
    machine::{GrooveMachineId, MelodyMachineId},
    midi::Note,
    TimeDivision,
};

pub fn wrapping_add(a: i32, b: i32, max: i32) -> i32 {
    let size = max + 1;
    ((a + b % size) + size) % size
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ParamValue {
    Number(u8),
    TimeDivision(TimeDivision),
    GrooveMachineId(GrooveMachineId),
    MelodyMachineId(MelodyMachineId),
    Note(Note),
}

impl Display for ParamValue {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            ParamValue::Number(num) => Display::fmt(&num, f),
            ParamValue::TimeDivision(time_div) => Display::fmt(&time_div, f),
            ParamValue::GrooveMachineId(id) => Display::fmt(&id, f),
            ParamValue::MelodyMachineId(id) => Display::fmt(&id, f),
            ParamValue::Note(note) => Display::fmt(&note, f),
        }
    }
}

impl From<ParamValue> for i32 {
    fn from(value: ParamValue) -> i32 {
        match value {
            ParamValue::Number(num) => num as i32,
            ParamValue::TimeDivision(time_div) => time_div as i32,
            ParamValue::GrooveMachineId(id) => id as i32,
            ParamValue::MelodyMachineId(id) => id as i32,
            ParamValue::Note(note) => note as i32,
        }
    }
}

type ParamName = String<6>;

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

    pub fn new_groove_machine_id_param(name: &str) -> Param {
        Param {
            name: name.into(),
            value: ParamValue::GrooveMachineId(GrooveMachineId::default()),
            min: ParamValue::GrooveMachineId(GrooveMachineId::Unit),
            max: ParamValue::GrooveMachineId(GrooveMachineId::Euclid),
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

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn value(&self) -> ParamValue {
        self.value.clone()
    }

    pub fn increment(&mut self, n: i32) {
        let value_i32: i32 = self.value.into();
        let min_i32: i32 = self.min.into();
        let max_i32: i32 = self.max.into();
        let new_value = (wrapping_add(value_i32 - min_i32, n, max_i32 - min_i32) + min_i32) as u8;
        self.value = match self.value {
            ParamValue::Number(_) => ParamValue::Number(new_value),
            ParamValue::TimeDivision(_) => ParamValue::TimeDivision(new_value.try_into().unwrap()),
            ParamValue::GrooveMachineId(_) => {
                ParamValue::GrooveMachineId(new_value.try_into().unwrap())
            }
            ParamValue::MelodyMachineId(_) => {
                ParamValue::MelodyMachineId(new_value.try_into().unwrap())
            }
            ParamValue::Note(_) => ParamValue::Note(new_value.try_into().unwrap()),
        }
    }
}

pub type ParamList = Vec<Box<Param>, 6>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn param_with_out_of_bounds_default_should_panic() {
        let _ = Param::new_number_param("NUM", 1, 10, 0);
    }

    #[test]
    fn param_number_should_increment() {
        let mut param_number = Param::new_number_param("NUM", 0, 10, 0);
        param_number.increment(1);
        match param_number.value() {
            ParamValue::Number(i) => assert_eq!(1, i),
            _ => panic!("unexpected param value"),
        }
    }

    #[test]
    fn param_number_starting_at_1_should_increment() {
        let mut param_number = Param::new_number_param("NUM", 1, 10, 1);
        param_number.increment(1);
        match param_number.value() {
            ParamValue::Number(i) => assert_eq!(2, i),
            _ => panic!("unexpected param value"),
        }
        param_number.increment(10);
        match param_number.value() {
            ParamValue::Number(i) => assert_eq!(2, i),
            _ => panic!("unexpected param value"),
        }
        param_number.increment(-5);
        match param_number.value() {
            ParamValue::Number(i) => assert_eq!(7, i),
            _ => panic!("unexpected param value"),
        }
    }

    #[test]
    fn param_time_division_should_increment() {
        let mut param_time_div = Param::new_time_division_param("SPD");
        param_time_div.increment(1);
        assert_param_time_division(TimeDivision::Eigth, &param_time_div);
        param_time_div.increment(9);
        assert_param_time_division(TimeDivision::Sixteenth, &param_time_div);
        param_time_div.increment(-1);
        assert_param_time_division(TimeDivision::ThirtySecond, &param_time_div);
        param_time_div.increment(-11);
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

    fn assert_param_time_division(expected: TimeDivision, param: &Param) {
        match param.value() {
            ParamValue::TimeDivision(time_div) => assert_eq!(expected, time_div),
            other => panic!("unexpected param value {:?}", other),
        }
    }
}
