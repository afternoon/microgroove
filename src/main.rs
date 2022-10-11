#![no_std]
#![no_main]

// show exit code and print panic message through defmt
use panic_probe as _;

mod microgroove {
    // rtic app module
    #[rtic::app(device = rp_pico::hal::pac, peripherals = true)]
    mod app {
        // hal aliases
        use rp_pico::hal;
        use rp_pico::hal::timer::Alarm;
        // use embedded_hal::digital::v2::InputPin;

        // defmt rtt logging (read the logs with cargo embed)
        use defmt;
        use defmt::{info, trace};
        use defmt_rtt as _;

        // time manipulation
        use fugit::MicrosDurationU32;

        // alloc-free data structures
        // use heapless::Vec;

        // how often to scan the MIDI in channel for messages
        // MIDI transmits data at 31250 baud, which is 3.9 bytes/ms
        // from the datasheet, rp2040 uarts have 32-byte FIFOs
        // in the worst case, our FIFO will fill in about 8ms
        // lets check it every 5ms to be safe
        const MIDI_IN_SCAN_INTERVAL_US: MicrosDurationU32 = MicrosDurationU32::millis(5);

        // rtic shared resources
        #[shared]
        struct Shared {
            // pins for buttons
            // pins for encoders
            // i2c/display interface
            midi_in_scan_alarm: hal::timer::Alarm0,
        }

        // rtic local resources
        #[local]
        struct Local {
            // ...
        }

        // rtic init
        #[init]
        fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
            // release spinlocks to avoid a deadlock after soft-reset
            unsafe {
                hal::sio::spinlock_reset();
            }

            // clock setup for timers and alarms
            let mut resets = ctx.device.RESETS;
            let mut watchdog = hal::Watchdog::new(ctx.device.WATCHDOG);
            let _clocks = hal::clocks::init_clocks_and_plls(
                rp_pico::XOSC_CRYSTAL_FREQ,
                ctx.device.XOSC,
                ctx.device.CLOCKS,
                ctx.device.PLL_SYS,
                ctx.device.PLL_USB,
                &mut resets,
                &mut watchdog,
            )
            .ok()
            .expect("init_clocks_and_plls should succeed");

            // schedule hardware alarm to check midi in
            let mut timer = hal::Timer::new(ctx.device.TIMER, &mut resets);
            let mut midi_in_scan_alarm = timer.alarm_0().unwrap();
            let _ = midi_in_scan_alarm.schedule(MIDI_IN_SCAN_INTERVAL_US);
            midi_in_scan_alarm.enable_interrupt();

            // TODO configure interrupts on button and encoder gpio pins
            // let mut buttons = Vec::<InputPin, 4>::new();
            // for button in buttons {
            //     button.set_interrupt_enabled(EdgeLow, true);
            // }

            // TODO init display
            
            info!("init complete");

            (Shared { midi_in_scan_alarm }, Local {}, init::Monotonics())
        }

        #[task(binds = IO_IRQ_BANK0, priority = 4)]
        fn io_irq_bank0(_ctx: io_irq_bank0::Context) {
            trace!("a wild gpio interupt has fired!");

            // TODO for each pin, check pin.interrupt_status(EdgeLow) to see if we fired
            // TODO for button presses, trigger an input mode change
            // TODO for encoders, trigger a param change
            // TODO clear_interrupt(EdgeLow)
        }

        #[task(binds = TIMER_IRQ_0, priority = 3, shared = [midi_in_scan_alarm])]
        fn timer_irq(mut ctx: timer_irq::Context) {
            // check midi input for messages
            trace!("checking for midi in");

            // spawn task(s) to tick tracks and potentially generate midi output

            // clear interupt and schedule next tick
            ctx.shared.midi_in_scan_alarm.lock(|alarm| {
                alarm.clear_interrupt();
                let _ = alarm.schedule(MIDI_IN_SCAN_INTERVAL_US);
            });
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
