extern crate alloc;

use core::fmt::Debug;
use heapless::String;

use crate::{params::ParamList, Sequence};

pub mod unit_machine;

pub trait Machine: Debug + Send {
    fn name(&self) -> &str;
    fn apply(&self, sequence: Sequence) -> Sequence;
    fn params(&self) -> &ParamList;
    fn params_mut(&mut self) -> &mut ParamList;
}

pub const GROOVE_MACHINE_IDS: &str = "UNIT";

pub const MELODY_MACHINE_IDS: &str = "UNIT";

pub fn machine_from_id(id: &str) -> Option<impl Machine> {
    let id_upcase = String::<6>::from(id).to_uppercase();
    match id_upcase.as_str() {
        "UNIT" => Some(unit_machine::UnitMachine::new()),
        _ => None,
    }
}
