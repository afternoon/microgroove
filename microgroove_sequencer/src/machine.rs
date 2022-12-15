extern crate alloc;

use alloc::boxed::Box;
use core::fmt::Debug;
use heapless::{String, Vec};

use crate::{machine_resources::MachineResources, param::ParamList, Sequence};

pub mod rand_melody_machine;
pub mod unit_machine;

use rand_melody_machine::RandMelodyMachine;
use unit_machine::UnitMachine;

#[derive(Debug)]
pub enum MachineError {
    UnknowMachine(String<6>),
}

/// A `Machine` represents a sequence generator that can be controlled via a list of parameters. In
/// Microgroove, each `Track` has 2 machines, one to generate the rhythm, one for the melody.
pub trait Machine: Debug + Send {
    fn name(&self) -> &str;
    fn apply(&self, sequence: Sequence, machine_resources: &mut MachineResources) -> Sequence;
    fn params(&self) -> &ParamList;
    fn params_mut(&mut self) -> &mut ParamList;
}

pub fn groove_machine_ids() -> Vec<String<6>, 10> {
    "UNIT".split_whitespace().map(|s| s.into()).collect()
}

pub fn melody_machine_ids() -> Vec<String<6>, 10> {
    "UNIT RAND".split_whitespace().map(|s| s.into()).collect()
}

// TODO impl TryFrom
pub fn machine_from_id(id: &str) -> Result<Box<dyn Machine>, MachineError> {
    let id_upcase = String::<6>::from(id).to_uppercase();
    match id_upcase.as_str() {
        "UNIT" => Ok(Box::new(UnitMachine::new())),
        "RAND" => Ok(Box::new(RandMelodyMachine::new())),
        unexpected => Err(MachineError::UnknowMachine(String::from(unexpected))),
    }
}
