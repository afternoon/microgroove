/// Model parameters as mutable values with metadata (name)
extern crate alloc;

use alloc::boxed::Box;
use core::cmp::PartialEq;
use core::fmt::{Debug, Display, Formatter, Result as FmtResult};
use heapless::{String, Vec};

use crate::{midi::Note, TimeDivision};

pub fn wrapping_add(a: i32, b: i32, max: i32) -> i32 {
    let size = max + 1;
    ((a + b % size) + size) % size
}

#[derive(Debug)]
pub enum ParamError {
    OptionsDoesNotContainValue,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ParamValue {
    Number(u8),
    TimeDivision(TimeDivision),
    GrooveMachine(String<6>),
    MelodyMachine(String<6>),
    Note(Note),
}

impl Display for ParamValue {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            ParamValue::Number(num) => Display::fmt(&num, f),
            ParamValue::TimeDivision(time_div) => Display::fmt(&time_div, f),
            ParamValue::GrooveMachine(id) => Display::fmt(&id, f),
            ParamValue::MelodyMachine(id) => Display::fmt(&id, f),
            ParamValue::Note(note) => Display::fmt(&note, f),
        }
    }
}

type ParamName = String<6>;
type ParamOptions = Vec<ParamValue, 128>;

#[derive(Clone, Debug)]
pub struct Param {
    name: ParamName,
    value: ParamValue,
    options: ParamOptions,
    value_pos: usize,
}

impl Param {
    pub fn new(
        name: ParamName,
        value: ParamValue,
        options: ParamOptions,
    ) -> Result<Param, ParamError> {
        let value_pos = options
            .iter()
            .position(|x| *x == value)
            .ok_or(ParamError::OptionsDoesNotContainValue)?;
        Ok(Param {
            name,
            value,
            options,
            value_pos,
        })
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn value(&self) -> ParamValue {
        self.value.clone()
    }

    pub fn value_position(&self) -> usize {
        self.value_pos
    }

    pub fn increment(&mut self, n: i32) {
        self.value_pos =
            wrapping_add(self.value_pos as i32, n, self.options.len() as i32 - 1) as usize;
        self.value = self.options[self.value_pos].clone();
    }
}

pub type ParamList = Vec<Box<Param>, 6>;

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_param_time_division(expected: TimeDivision, param: &Param) {
        match param.value() {
            ParamValue::TimeDivision(time_div) => assert_eq!(expected, time_div),
            other => panic!("unexpected param value {:?}", other),
        }
    }

    #[test]
    fn param_number_should_increment() {
        let mut param_number = Param::new(
            String::from("NUM"),
            ParamValue::Number(0),
            (0..=10).map(ParamValue::Number).collect::<ParamOptions>(),
        )
        .unwrap();
        param_number.increment(1);
        match param_number.value() {
            ParamValue::Number(i) => assert_eq!(1, i),
            _ => panic!("unexpected param value"),
        }
    }

    #[test]
    fn param_time_division_should_increment() {
        let mut param_test_enum = Param::new(
            "TEST".into(),
            ParamValue::TimeDivision(TimeDivision::Sixteenth),
            TimeDivision::all_variants()
                .iter()
                .map(|&time_div| ParamValue::TimeDivision(time_div))
                .collect(),
        )
        .unwrap();
        param_test_enum.increment(1);
        assert_param_time_division(TimeDivision::Eigth, &param_test_enum);
        param_test_enum.increment(9);
        assert_param_time_division(TimeDivision::Sixteenth, &param_test_enum);
        param_test_enum.increment(-1);
        assert_param_time_division(TimeDivision::ThirtySecond, &param_test_enum);
        param_test_enum.increment(-11);
        assert_param_time_division(TimeDivision::Whole, &param_test_enum);
    }

    #[test]
    fn param_enum_value_should_have_to_string() {
        let param_test_enum = Param::new(
            "TEST".into(),
            ParamValue::TimeDivision(TimeDivision::Sixteenth),
            TimeDivision::all_variants()
                .iter()
                .map(|&time_div| ParamValue::TimeDivision(time_div))
                .collect(),
        )
        .unwrap();
        match param_test_enum.value() {
            ParamValue::TimeDivision(time_division) => {
                assert_eq!("1/16", time_division.to_string())
            }
            _ => panic!("unexpected param value"),
        }
    }

    #[test]
    fn param_list_can_store_different_param_types() {
        let param_number = Param::new(
            String::from("NUM"),
            ParamValue::Number(0),
            (0..=10).map(ParamValue::Number).collect::<ParamOptions>(),
        )
        .unwrap();
        let param_test_enum = Param::new(
            "TEST".into(),
            ParamValue::TimeDivision(TimeDivision::Sixteenth),
            TimeDivision::all_variants()
                .iter()
                .map(|&time_div| ParamValue::TimeDivision(time_div))
                .collect(),
        )
        .unwrap();
        let _param_list =
            ParamList::from_slice(&[Box::new(param_number), Box::new(param_test_enum)]);
    }
}
