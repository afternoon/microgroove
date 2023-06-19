pub mod positional_encoder {
    use core::fmt::Debug;
    use defmt::{error, trace};
    use rotary_encoder_hal::{Direction, Rotary};
    use rp_pico::hal::gpio::DynPin;

    pub struct PositionalEncoder {
        encoder: Rotary<DynPin, DynPin>,
        value: i8,
    }

    impl PositionalEncoder {
        pub fn new(mut pin_a: DynPin, mut pin_b: DynPin) -> PositionalEncoder {
            pin_a.into_pull_up_input();
            pin_b.into_pull_up_input();
            PositionalEncoder {
                encoder: Rotary::new(pin_a.into(), pin_b.into()),
                value: 0,
            }
        }

        /// Check the encoder state for changes. This should be called frequently, e.g.
        /// every 1ms. Returns a `Some` containing the encoder value if there have been
        /// changes, `None` otherwise.
        pub fn update(&mut self) -> Option<i8> {
            match self.encoder.update() {
                Ok(Direction::Clockwise) => {
                    trace!("[PositionalEncoder::update] Direction::Clockwise");
                    self.value += 1;
                    Some(self.value)
                }
                Ok(Direction::CounterClockwise) => {
                    trace!("[PositionalEncoder::update] Direction::CounterClockwise");
                    self.value -= 1;
                    Some(self.value)
                }
                Ok(Direction::None) => None,
                Err(_error) => {
                    error!("[PositionalEncoder::update] could not update encoder");
                    None
                }
            }
        }

        /// Get the value of the encoder, and then reset that to zero. This has the
        /// semantics of "I would like to know your value, which I will use to update my
        /// state, so you can then discard it."
        pub fn take_value(&mut self) -> Option<i8> {
            let value = self.value;
            if value == 0 {
                None
            } else {
                self.value = 0;
                Some(value)
            }
        }
    }

    impl Debug for PositionalEncoder {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            write!(f, "encoder")
        }
    }
}

pub mod encoder_array {
    use super::positional_encoder::PositionalEncoder;
    use heapless::Vec;

    pub const ENCODER_COUNT: usize = 6;

    /// An array of multiple `PositionalEncoders`.
    pub struct EncoderArray {
        encoders: Vec<PositionalEncoder, ENCODER_COUNT>,
    }

    impl EncoderArray {
        pub fn new(encoders: Vec<PositionalEncoder, ENCODER_COUNT>) -> EncoderArray {
            EncoderArray { encoders }
        }

        pub fn update(&mut self) -> Option<()> {
            let any_changes = self
                .encoders
                .iter_mut()
                .map(|enc| enc.update())
                .any(|opt| opt.is_some());
            if any_changes {
                Some(())
            } else {
                None
            }
        }

        pub fn take_values(&mut self) -> Vec<Option<i8>, ENCODER_COUNT> {
            self.encoders
                .iter_mut()
                .map(|enc| enc.take_value())
                .collect()
        }
    }
}
