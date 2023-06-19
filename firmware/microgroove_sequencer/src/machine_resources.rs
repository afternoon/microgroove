#[cfg(feature = "target_release")]
use rp2040_hal::rosc::{Enabled, RingOscillator};

#[cfg(feature = "host_testing")]
use rand::prelude::*;

#[cfg(feature = "target_release")]
use rand_core::RngCore;

/// `MachineResources` defines a set of methods that machines can use when generating sequences,
/// e.g a source of random numbers.
pub struct MachineResources {
    #[cfg(feature = "target_release")]
    rosc: RingOscillator<Enabled>,
}

impl MachineResources {
    #[cfg(feature = "target_release")]
    pub fn new(rosc: RingOscillator<Enabled>) -> MachineResources {
        MachineResources { rosc }
    }

    #[cfg(feature = "host_testing")]
    pub fn new() -> MachineResources {
        MachineResources {}
    }

    #[cfg(feature = "target_release")]
    pub fn random_u64(&mut self) -> u64 {
        self.rosc.next_u64()
    }

    #[cfg(feature = "host_testing")]
    pub fn random_u64(&mut self) -> u64 {
        random()
    }
}
