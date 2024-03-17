use alloc::boxed::Box;
use core::fmt::{Debug, Display, Formatter, Result as FmtResult};
use heapless::String;

use crate::{machine_resources::MachineResources, param::ParamList, Sequence};

pub mod euclidean_rhythm_machine;
pub mod grids_rhythm_machine;
pub mod rand_melody_machine;
pub mod unit_machine;

use euclidean_rhythm_machine::EuclideanRhythmMachine;
use grids_rhythm_machine::GridsRhythmMachine;
use rand_melody_machine::RandMelodyMachine;
use unit_machine::UnitMachine;

#[derive(Debug)]
pub enum MachineError {
    UnknowMachine(String<6>),
}

/// A `Machine` represents a sequence generator that can be controlled via a list of parameters. In
/// Microgroove, each `Track` has 2 machines, one to generate the rhythm, one for the melody.
pub trait Machine: Debug + Send {
    fn name(&self) -> &str; // TODO redundant because Display implmented for machine IDs
    fn generate(&mut self, machine_resources: &mut MachineResources);
    fn apply(&self, sequence: Sequence) -> Sequence;
    fn params(&self) -> &ParamList;
    fn params_mut(&mut self) -> &mut ParamList;
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum RhythmMachineId {
    Unit,
    #[default]
    Euclid,
    Grids,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum MelodyMachineId {
    Unit,
    #[default]
    Rand,
}

impl From<RhythmMachineId> for Box<dyn Machine> {
    fn from(value: RhythmMachineId) -> Self {
        match value {
            RhythmMachineId::Unit => Box::new(UnitMachine::new()),
            RhythmMachineId::Euclid => Box::new(EuclideanRhythmMachine::new()),
            RhythmMachineId::Grids => Box::new(GridsRhythmMachine::new()),
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

impl Display for RhythmMachineId {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            RhythmMachineId::Unit => Display::fmt("UNIT", f),
            RhythmMachineId::Euclid => Display::fmt("EUCLID", f),
            RhythmMachineId::Grids => Display::fmt("GRIDS", f),
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

impl TryFrom<u8> for RhythmMachineId {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(RhythmMachineId::Unit),
            1 => Ok(RhythmMachineId::Euclid),
            2 => Ok(RhythmMachineId::Grids),
            _ => Err(()),
        }
    }
}

impl TryFrom<u8> for MelodyMachineId {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(MelodyMachineId::Unit),
            1 => Ok(MelodyMachineId::Rand),
            _ => Err(()),
        }
    }
}
