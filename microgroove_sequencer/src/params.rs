/// Model parameters as mutable values with metadata (name)
extern crate alloc;

use alloc::boxed::Box;
use core::fmt::{Debug, Write};
use heapless::{String, Vec};

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
        NumberParam { name: name.into(), val: initial, min, max }
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
        if self.val < self.min { self.val = self.min; }
        else if self.val > self.max { self.val = self.max; }
    }

    fn value_i8(&self) -> Option<i8> {
        Some(self.val)
    }
}
