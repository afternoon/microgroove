/// Model parameters as mutable values with metadata (name)
extern crate alloc;

use alloc::boxed::Box;
use core::fmt::{Debug, Write};
use heapless::{String, Vec};

pub fn wrapping_add(a: i32, b: i32, max: i32) -> i32 {
    let size = max + 1;
    ((a + b % size) + size) % size
}

pub trait Param: Debug + Send {
    fn name(&self) -> &str {
        "DISABLED"
    }
    fn increment(&mut self, n: i8);
    fn value_str(&self) -> String<10>;
    fn value_i8(&self) -> Option<i8> {
        None
    }
}

pub trait ParamAdapter {
    fn apply(&mut self) {}
}

pub type ParamList = Vec<Box<dyn Param>, 6>;

#[derive(Clone, Debug)]
pub struct NumberParam {
    name: String<6>,
    val: i8,
    min: i8,
    max: i8,
}

impl NumberParam {
    pub fn new(name: &str, min: i8, max: i8, initial: i8) -> NumberParam {
        NumberParam {
            name: name.into(),
            val: initial,
            min,
            max,
        }
    }
}

impl Param for NumberParam {
    fn name(&self) -> &str {
        self.name.as_str()
    }

    fn value_str(&self) -> String<10> {
        let mut val_string = String::<10>::new();
        let _ = write!(val_string, "{}", self.val);
        val_string
    }

    fn increment(&mut self, n: i8) {
        self.val += n;
        if self.val < self.min {
            self.val = self.min;
        } else if self.val > self.max {
            self.val = self.max;
        }
    }

    fn value_i8(&self) -> Option<i8> {
        Some(self.val)
    }
}

pub type Options = Vec<String<4>, 20>;

#[derive(Clone, Debug)]
pub struct EnumParam {
    name: String<6>,
    options: Options,
    index: usize,
}

/// Param for selecting from a set of available choices. Options are defined as a `&str` of
/// whitespace-separated tokens, e.g. `"1st 2nd 3rd 4th"`. Options are represented as `String<6>` so
/// that many different types of choice can be represented easily, and to avoid coupling the param
/// implementation to the underlying data model. You should be careful to keep the list of options
/// in sync with whatever values it relates to.
impl EnumParam {
    pub fn new(name: &str, options_space_separated: &str, initial: Option<&str>) -> EnumParam {
        let mut options = Options::new();
        for option_str in options_space_separated.split_ascii_whitespace() {
            options.push(option_str.into()).unwrap();
        }
        let index = initial
            .map(|initial| options.iter().position(|x| x == initial).unwrap_or_default())
            .unwrap_or_default();
        EnumParam {
            name: name.into(),
            options,
            index,
        }
    }
}

impl Param for EnumParam {
    fn name(&self) -> &str {
        self.name.as_str()
    }

    fn value_str(&self) -> String<10> {
        self.options[self.index].as_str().into()
    }

    /// Scroll through the available options. Incrementing wraps around once the user scrolls past
    /// the first and last options.
    fn increment(&mut self, n: i8) {
        self.index =
            wrapping_add(self.index as i32, n as i32, (self.options.len() - 1) as i32) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn number_param_should_calculate_increments_correctly() {
        let mut param = NumberParam::new("TEST", 1, 16, 1);
        assert_eq!(1, param.value_i8().unwrap());
        param.increment(1);
        assert_eq!(2, param.value_i8().unwrap());
        param.increment(15);
        assert_eq!(16, param.value_i8().unwrap());
        param.increment(1);
        assert_eq!(16, param.value_i8().unwrap());
        param.increment(-16);
        assert_eq!(1, param.value_i8().unwrap());
        param.increment(-1);
        assert_eq!(1, param.value_i8().unwrap());
    }

    #[test]
    fn enum_param_should_calculate_small_increments_correctly() {
        let mut param = EnumParam::new("TEST", "1 1/4 1/8 1/16 1/32", None);
        assert_eq!("1", param.value_str());
        param.increment(1);
        assert_eq!("1/4", param.value_str());
        param.increment(-1);
        assert_eq!("1", param.value_str());
        param.increment(-1);
        assert_eq!("1/32", param.value_str());
        param.increment(1);
        assert_eq!("1", param.value_str());
    }

    #[test]
    fn enum_param_should_calculate_large_increments_correctly() {
        let mut param = EnumParam::new("TEST", "1 1/4 1/8 1/16 1/32", None);
        assert_eq!("1", param.value_str());
        param.increment(21);
        assert_eq!("1/4", param.value_str());
        param.increment(-21);
        assert_eq!("1", param.value_str());
    }

    #[test]
    fn enum_param_should_have_initial_value() {
        let param = EnumParam::new("TEST", "1 1/4 1/8 1/16 1/32", Some("1/16"));
        assert_eq!("1/16", param.value_str());
    }

    #[test]
    fn enum_param_should_ignore_invalid_initial_value() {
        let param = EnumParam::new("TEST", "1 1/4 1/8 1/16 1/32", Some("XYZ"));
        assert_eq!("1", param.value_str());
    }
}
