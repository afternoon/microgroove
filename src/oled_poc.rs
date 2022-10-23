// Resources:
// 
// - Buttons and LEDs: https://github.com/rp-rs/rp-hal/blob/main/boards/rp-pico/examples/pico_gpio_in_out.rs
// - MIDI: https://github.com/rust-midi/embedded-midi/blob/main/bluepill-examples/examples/passthrough.rs
// - SSD1306/graphics: https://github.com/rp-rs/rp-hal/blob/main/boards/rp-pico/examples/pico_i2c_oled_display_ssd1306.rs
// - Interrupts/timing: https://github.com/rp-rs/rp-hal/blob/main/boards/rp-pico/examples/pico_rtic.rs
// - https://rtic.rs/1/book/en/
// - Rotary encoders: https://github.com/leshow/rotary-encoder-hal
//

#![no_std]
#![no_main]

// show exit code and print panic message through defmt
use panic_halt as _;

// logging
use defmt;
use defmt::debug;
use defmt_rtt as _;

// embedded entry point macro
use rp_pico::entry;

// hal aliases
use rp_pico::hal;
use rp_pico::hal::pac;

// rotary encoder driver
use rotary_encoder_hal::{Direction, Rotary};

// ssd1306 oled display driver
use ssd1306::{prelude::*, Ssd1306};

// allocation-free data structures
use heapless;
use core::fmt::Write;

// graphics
use embedded_graphics::{
    mono_font::{ascii::FONT_4X6, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};

// time handling traits
use fugit::RateExtU32;

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();

    // clock setup
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);
    let clocks = hal::clocks::init_clocks_and_plls(
        rp_pico::XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    // pin setup
    let sio = hal::sio::Sio::new(pac.SIO);
    let pins = rp_pico::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // setup rotary encoder
    let pin_a = pins.gpio9.into_pull_up_input();
    let pin_b = pins.gpio10.into_pull_up_input();
    let mut enc = Rotary::new(pin_a, pin_b);

    // configure i2c pins
    let sda_pin = pins.gpio26.into_mode::<hal::gpio::FunctionI2C>();
    let scl_pin = pins.gpio27.into_mode::<hal::gpio::FunctionI2C>();

    // create i2c driver
    let i2c = hal::I2C::i2c1(
        pac.I2C1,
        sda_pin,
        scl_pin,
        1.MHz(),
        &mut pac.RESETS,
        &clocks.peripheral_clock,
    );

    // create i2c display interface
    let mut display = Ssd1306::new(
        ssd1306::I2CDisplayInterface::new_alternate_address(i2c),
        DisplaySize128x64,
        DisplayRotation::Rotate0).into_buffered_graphics_mode();

    // intialise display
    display.init().unwrap();

    debug!("display initialised");

    // define style for text
    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_4X6)
        .text_color(BinaryColor::On)
        .build();

    let mut pos: isize = 0;
    let mut pos_string: heapless::String<8> = heapless::String::new();

    loop {
        match enc.update().unwrap() {
            Direction::Clockwise => {
                pos += 1;
            }
            Direction::CounterClockwise => {
                pos -= 1;
            }
            Direction::None => {}
        }

        debug!("encoder pos={}", pos);

        display.clear();

        Text::with_baseline("MICROGROOVE", Point::zero(), text_style, Baseline::Top)
            .draw(&mut display)
            .unwrap();

        let _ = write!(pos_string, "POS: {}", pos);
        Text::with_baseline(pos_string.as_str(), Point::new(0, 6), text_style, Baseline::Top)
            .draw(&mut display)
            .unwrap();

        display.flush().unwrap();
    }
}
