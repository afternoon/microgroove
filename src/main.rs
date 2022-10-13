#![no_std]
#![no_main]

// use probe_run to print panic messages
use panic_probe as _;

mod microgroove {
    // rtic app module
    #[rtic::app(device = rp_pico::hal::pac, peripherals = true, dispatchers = [PIO0_IRQ_0, PIO0_IRQ_1, PIO1_IRQ_0, PIO1_IRQ_1])]
    mod app {
        // hal aliases
        use rp_pico::hal;
        use rp_pico::hal::{gpio, pac, uart};
        use gpio::pin::bank0::{Gpio0, Gpio1};
        use rp_pico::hal::Clock;

        // embedded midi crate
        use embedded_midi;
        use midi_types::MidiMessage;

        /// alias the type for our UART pins
        type MidiInUartPin = gpio::Pin<Gpio0, gpio::Function<gpio::Uart>>;
        type MidiOutUartPin = gpio::Pin<Gpio1, gpio::Function<gpio::Uart>>;
        type MidiUartPins = (MidiInUartPin, MidiOutUartPin);

        // non-blocking io
        use nb::block;

        // time manipulation
        use fugit;

        // defmt rtt logging (read the logs with cargo embed, etc)
        use defmt;
        use defmt::{info, debug, trace};
        use defmt_rtt as _;

        // rtic shared resources
        #[shared]
        struct Shared {
            // pins for buttons
            // pins for encoders
            // i2c/display interface
        }

        // rtic local resources
        #[local]
        struct Local {
            midi_in: embedded_midi::MidiIn<uart::Reader<pac::UART0, MidiUartPins>>,
            midi_out: embedded_midi::MidiOut<uart::Writer<pac::UART0, MidiUartPins>>,
        }

        // rtic init
        #[init]
        fn init(mut ctx: init::Context) -> (Shared, Local, init::Monotonics) {
            // release spinlocks to avoid a deadlock after soft-reset
            unsafe {
                hal::sio::spinlock_reset();
            }

            // clock setup for timers and alarms
            let mut watchdog = hal::Watchdog::new(ctx.device.WATCHDOG);
            let clocks = hal::clocks::init_clocks_and_plls(
                rp_pico::XOSC_CRYSTAL_FREQ,
                ctx.device.XOSC,
                ctx.device.CLOCKS,
                ctx.device.PLL_SYS,
                ctx.device.PLL_USB,
                &mut ctx.device.RESETS,
                &mut watchdog,
            )
            .ok()
            .expect("init_clocks_and_plls should succeed");

            // the single-cycle i/o block controls our gpio pins
            let sio = hal::Sio::new(ctx.device.SIO);

            // set the pins to their default state
            let pins = rp_pico::Pins::new(
                ctx.device.IO_BANK0,
                ctx.device.PADS_BANK0,
                sio.gpio_bank0,
                &mut ctx.device.RESETS,
            );

            // put pins for midi into uart mode
            let midi_uart_pins = (
                // UART TX (characters sent from RP2040) on pin 1 (GPIO0)
                pins.gpio0.into_mode::<gpio::FunctionUart>(),
                // UART RX (characters received by RP2040) on pin 2 (GPIO1)
                pins.gpio1.into_mode::<gpio::FunctionUart>(),
            );

            // configure uart comms settings
            let mut uart_config_31250_8_N_1 = uart::common_configs::_38400_8_N_1;
            uart_config_31250_8_N_1.baudrate = fugit::HertzU32::from_raw(31250);

            // make a uart peripheral on the given pins
            let mut midi_uart = uart::UartPeripheral::new(ctx.device.UART0, midi_uart_pins, &mut ctx.device.RESETS)
                .enable(
                    uart_config_31250_8_N_1,
                    clocks.peripheral_clock.freq(),
                )
                .expect("uart enable should succeed");

            // configure uart interrupt to fire on midi input
            midi_uart.enable_rx_interrupt();

            let (midi_reader, midi_writer) = midi_uart.split();

            let midi_in = embedded_midi::MidiIn::new(midi_reader);
            let midi_out = embedded_midi::MidiOut::new(midi_writer);

            // TODO configure interrupts on button and encoder gpio pins
            // let mut buttons = Vec::<InputPin, 4>::new();
            // for button in buttons {
            //     button.set_interrupt_enabled(EdgeLow, true);
            // }

            // TODO init display
            
            info!("init complete");

            (Shared {}, Local { midi_in, midi_out }, init::Monotonics())
        }

        #[task(binds = IO_IRQ_BANK0, priority = 4)]
        fn io_irq_bank0(_ctx: io_irq_bank0::Context) {
            trace!("a wild gpio_bank0 interrupt has fired!");

            // TODO for each pin, check pin.interrupt_status(EdgeLow) to see if we fired
            // TODO for button presses, trigger an input mode change
            // TODO for encoders, trigger a param change
            // TODO clear_interrupt(EdgeLow)
        }

        #[task(binds = UART0_IRQ, priority = 4, local = [midi_in])]
        fn uart0_irq(ctx: uart0_irq::Context) {
            // check midi input for messages
            trace!("a wild uart0 interrupt has fired!");

            // read those sweet sweet midi bytes!
            if let Some(message) = block!(ctx.local.midi_in.read()).ok() {
                // log the message
                match message {
                    MidiMessage::TimingClock => debug!("got midi message: TimingClock"),
                    MidiMessage::Start => debug!("got midi message: Start"),
                    MidiMessage::Stop => debug!("got midi message: Stop"),
                    MidiMessage::Continue => debug!("got midi message: Continue"),
                    _ => debug!("got midi message: UNKNOWN"),
                }

                // pass received message to midi out ("soft thru")
                send_midi::spawn(message).expect("send_midi::spawn should succeed");

                // TODO if clock, spawn task(s) to tick tracks and potentially generate midi output

                // TODO handle start, stop, continue messages
            }

            // TODO clear interupt and schedule next tick
        }

        #[task(priority = 3, capacity = 50, local = [midi_out])]
        fn send_midi(ctx: send_midi::Context, message: MidiMessage) {
            ctx.local.midi_out.write(&message)
                .ok()
                .expect("midi_out.write should succeed");
        }

        // idle task needed, default idle task calls wfi(), which breaks rtt
        #[idle]
        fn taskmain(_: taskmain::Context) -> ! {
            loop {
                cortex_m::asm::nop();
            }
        }
    }
}
