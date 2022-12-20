use alloc::boxed::Box;
use core::fmt::{Debug, Display, Formatter, Result as FmtResult};
use heapless::String;

use crate::{machine_resources::MachineResources, param::ParamList, Sequence};

pub mod euclidean_groove_machine;
pub mod rand_melody_machine;
pub mod unit_machine;

use euclidean_groove_machine::EuclideanGrooveMachine;
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

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum GrooveMachineId {
    #[default]
    Unit,
    Euclid,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum MelodyMachineId {
    #[default]
    Unit,
    Rand
}

impl From<GrooveMachineId> for Box<dyn Machine> {
    fn from(value: GrooveMachineId) -> Self {
        match value {
            GrooveMachineId::Unit => Box::new(UnitMachine::new()),
            GrooveMachineId::Euclid => Box::new(EuclideanGrooveMachine::new()),
        }
    }
}

impl From<MelodyMachineId> for Box<dyn Machine> {
    fn from(value: MelodyMachineId) -> Self {
        match value {
            MelodyMachineId::Unit => Box::new(UnitMachine::new()),
            MelodyMachineId::Rand => Box::new(RandMelodyMachine::new()),
        }
    }
}

impl Display for GrooveMachineId {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            GrooveMachineId::Unit => Display::fmt("UNIT", f),
            GrooveMachineId::Euclid => Display::fmt("EUCLID", f),
        }
    }
}

impl Display for MelodyMachineId {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            MelodyMachineId::Unit => Display::fmt("UNIT", f),
            MelodyMachineId::Rand => Display::fmt("RAND", f),
        }
    }
}

impl TryFrom<u8> for GrooveMachineId {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(GrooveMachineId::Unit),
            1 => Ok(GrooveMachineId::Euclid),
            _ => Err(())
        }
    }
}

impl TryFrom<u8> for MelodyMachineId {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(MelodyMachineId::Unit),
            1 => Ok(MelodyMachineId::Rand),
            _ => Err(())
        }
    }
}
