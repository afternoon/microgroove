#![no_std]
#![no_main]

use panic_probe as _;

mod microgroove {
    // gfx module knows how to draw the on-screen graphics
    pub mod gfx {

    }

    // ui module handles input and calls gfx to output to the display
    pub mod ui {
        // enum for current UI page (controls what is displayed and which parameters are mapped to
        // encoders)
        pub enum InputMode {
            Track,
            Groove,
            Melody
        }
    }

    // rtic module runs our app as a set of concurrent tasks interfacing with the hardware
    #[rtic::app(
        device = rp_pico::hal::pac,
        peripherals = true,
    )]
    mod app {
        // hal aliases - turns out we have a big dependency on the hardware 😀
        use rp_pico::{
            hal::{
                gpio::{
                    Interrupt::EdgeLow,
                    Pin,
                    PullUpInput,
                    pin::bank0::{Gpio0, Gpio1, Gpio2},
                },
                sio::{
                    self,
                    Sio
                },
            },
            Pins,
        };

        // defmt rtt logging (read the logs with cargo embed, etc)
        use defmt;
        use defmt::{error, info, debug, trace};
        use defmt_rtt as _;

        // crate imports
        use super::ui::InputMode;

        // type alias for button pins
        type ButtonTrackPin = Pin<Gpio0, PullUpInput>;
        type ButtonGroovePin = Pin<Gpio1, PullUpInput>;
        type ButtonMelodyPin = Pin<Gpio2, PullUpInput>;

        // RTIC shared resources
        #[shared]
        struct Shared {
            // current page of the UI
            input_mode: InputMode,
        }

        // RTIC local resources
        #[local]
        struct Local {
            // pins for buttons
            button_track_pin: ButtonTrackPin,
            button_groove_pin: ButtonGroovePin,
            button_melody_pin: ButtonMelodyPin,
        }

        // RTIC init
        #[init]
        fn init(mut ctx: init::Context) -> (Shared, Local, init::Monotonics) {
            info!("hello world!");

            // release spinlocks to avoid a deadlock after soft-reset
            unsafe {
                sio::spinlock_reset();
            }

            // the single-cycle i/o block controls our gpio pins
            let sio = Sio::new(ctx.device.SIO);

            // set the pins to their default state
            let pins = Pins::new(
                ctx.device.IO_BANK0,
                ctx.device.PADS_BANK0,
                sio.gpio_bank0,
                &mut ctx.device.RESETS,
            );

            // configure interrupts on button and encoder GPIO pins
            let button_track_pin = pins.gpio0.into_pull_up_input();
            let button_groove_pin = pins.gpio1.into_pull_up_input();
            let button_melody_pin = pins.gpio2.into_pull_up_input();
            button_track_pin.set_interrupt_enabled(EdgeLow, true);
            button_groove_pin.set_interrupt_enabled(EdgeLow, true);
            button_melody_pin.set_interrupt_enabled(EdgeLow, true);

            // APP STATE
            
            // show track page of UI at startup
            let input_mode = InputMode::Track;

            // LET'S GOOOO!!

            info!("init: complete 🤘");

            (
                Shared {
                    input_mode,
                },
                Local {
                    button_track_pin,
                    button_groove_pin,
                    button_melody_pin,
                },
                init::Monotonics(),
            )
        }

        // handle button pin interrupts
        #[task(
            binds = IO_IRQ_BANK0,
            priority = 4,
            shared = [input_mode],
            local = [button_track_pin, button_groove_pin, button_melody_pin]
        )]
        fn io_irq_bank0(mut ctx: io_irq_bank0::Context) {
            trace!("a wild gpio_bank0 interrupt has fired!");

            // for each button, check interrupt status to see if we fired
            if ctx.local.button_track_pin.interrupt_status(EdgeLow) {
                info!("track button pressed");
                ctx.shared.input_mode.lock(|input_mode| {
                    *input_mode = InputMode::Track;
                });
                ctx.local.button_track_pin.clear_interrupt(EdgeLow);
            }
            if ctx.local.button_groove_pin.interrupt_status(EdgeLow) {
                info!("groove button pressed");
                ctx.shared.input_mode.lock(|input_mode| {
                    *input_mode = InputMode::Groove;
                });
                ctx.local.button_groove_pin.clear_interrupt(EdgeLow);
            }
            if ctx.local.button_melody_pin.interrupt_status(EdgeLow) {
                info!("melody button pressed");
                ctx.shared.input_mode.lock(|input_mode| {
                    *input_mode = InputMode::Melody;
                });
                ctx.local.button_melody_pin.clear_interrupt(EdgeLow);
            }
        }
    }
}
