/// Device initialisation and interfacing.
use super::encoder::{encoder_array::EncoderArray, positional_encoder::PositionalEncoder};
use embedded_midi;
use fugit::{HertzU32, RateExtU32};
use heapless::Vec;
use rp2040_hal::{clocks::PeripheralClock, rosc::Enabled};
use rp_pico::{
    hal::{
        clocks::{self, Clock},
        gpio::{
            pin::bank0::{Gpio0, Gpio1, Gpio16, Gpio17, Gpio2, Gpio26, Gpio27},
            FunctionI2C, FunctionUart, Pin, PullUpInput,
        },
        pac::{self, I2C1, RESETS, TIMER, UART0},
        rosc::RingOscillator,
        sio::Sio,
        timer::{monotonic::Monotonic, Alarm0},
        uart::{DataBits, Reader, StopBits, UartConfig, UartPeripheral, Writer},
        Timer, Watchdog, I2C,
    },
    Pins, XOSC_CRYSTAL_FREQ,
};
use ssd1306::{mode::BufferedGraphicsMode, prelude::*, I2CDisplayInterface, Ssd1306};

// type alias for UART pins
type MidiOutUartPin = Pin<Gpio16, FunctionUart>;
type MidiInUartPin = Pin<Gpio17, FunctionUart>;
type MidiUartPins = (MidiOutUartPin, MidiInUartPin);

// microgroove-specific midi in/out channel types
pub type MidiIn = embedded_midi::MidiIn<Reader<UART0, MidiUartPins>>;
pub type MidiOut = embedded_midi::MidiOut<Writer<UART0, MidiUartPins>>;

// type alias for display pins
type DisplaySdaPin = Pin<Gpio26, FunctionI2C>;
type DisplaySclPin = Pin<Gpio27, FunctionI2C>;
pub type DisplayPins = (DisplaySdaPin, DisplaySclPin);

// microgroove-specific display type
pub type Display = Ssd1306<
    I2CInterface<I2C<I2C1, DisplayPins>>,
    DisplaySize128x64,
    BufferedGraphicsMode<DisplaySize128x64>,
>;

// type alias for button pins
pub type ButtonTrackPin = Pin<Gpio0, PullUpInput>;
pub type ButtonRhythmPin = Pin<Gpio1, PullUpInput>;
pub type ButtonMelodyPin = Pin<Gpio2, PullUpInput>;
type ButtonArray = (ButtonTrackPin, ButtonRhythmPin, ButtonMelodyPin);

pub fn setup(
    mut pac: pac::Peripherals,
) -> (
    MidiIn,
    MidiOut,
    Display,
    ButtonArray,
    EncoderArray,
    RingOscillator<Enabled>,
    Monotonic<Alarm0>,
) {
    // setup gpio pins
    let sio = Sio::new(pac.SIO);
    let pins = Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // setup clocks
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let clocks = clocks::init_clocks_and_plls(
        XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .expect("init_clocks_and_plls(...) should succeed");

    // setup MIDI IO
    let (midi_in, midi_out) = new_midi_uart(
        pac.UART0,
        pins.gpio16.into_mode::<FunctionUart>(),
        pins.gpio17.into_mode::<FunctionUart>(),
        &mut pac.RESETS,
        clocks.peripheral_clock.freq(),
    );

    // setup display
    let display = new_display(
        pac.I2C1,
        pins.gpio26.into_mode::<FunctionI2C>(),
        pins.gpio27.into_mode::<FunctionI2C>(),
        &mut pac.RESETS,
        &clocks.peripheral_clock,
    );

    // setup buttons
    let button_track_pin = pins.gpio0.into_pull_up_input();
    let button_rhythm_pin = pins.gpio1.into_pull_up_input();
    let button_melody_pin = pins.gpio2.into_pull_up_input();
    let buttons = (button_track_pin, button_rhythm_pin, button_melody_pin);

    // setup encoders
    let mut encoder_vec = Vec::new();
    encoder_vec
        .push(PositionalEncoder::new(
            pins.gpio9.into(),
            pins.gpio10.into(),
        ))
        .expect("encoder_vec.push(...) should succeed");
    encoder_vec
        .push(PositionalEncoder::new(
            pins.gpio11.into(),
            pins.gpio12.into(),
        ))
        .expect("encoder_vec.push(...) should succeed");
    encoder_vec
        .push(PositionalEncoder::new(
            pins.gpio13.into(),
            pins.gpio14.into(),
        ))
        .expect("encoder_vec.push(...) should succeed");
    encoder_vec
        .push(PositionalEncoder::new(pins.gpio3.into(), pins.gpio4.into()))
        .expect("encoder_vec.push(...) should succeed");
    encoder_vec
        .push(PositionalEncoder::new(pins.gpio5.into(), pins.gpio6.into()))
        .expect("encoder_vec.push(...) should succeed");
    encoder_vec
        .push(PositionalEncoder::new(pins.gpio7.into(), pins.gpio8.into()))
        .expect("encoder_vec.push(...) should succeed");
    let encoders = EncoderArray::new(encoder_vec);

    // create a ring oscillator for random-number generation
    let rosc = RingOscillator::new(pac.ROSC).initialize();

    (
        midi_in,
        midi_out,
        display,
        buttons,
        encoders,
        rosc,
        new_monotonic_timer(pac.TIMER, &mut pac.RESETS),
    )
}

fn new_monotonic_timer(timer: TIMER, resets: &mut RESETS) -> Monotonic<Alarm0> {
    // setup monotonic timer for rtic
    let mut timer = Timer::new(timer, resets);
    let monotonic_alarm = timer.alarm_0().expect("should get alarm_0");
    Monotonic::new(timer, monotonic_alarm)
}

fn new_midi_uart(
    uart: UART0,
    out_pin: MidiOutUartPin,
    in_pin: MidiInUartPin,
    resets: &mut RESETS,
    peripheral_clock_freq: HertzU32,
) -> (MidiIn, MidiOut) {
    let midi_uart_pins = (out_pin, in_pin);
    let uart_config = UartConfig::new(31_250.Hz(), DataBits::Eight, None, StopBits::One);
    let mut midi_uart = UartPeripheral::new(uart, midi_uart_pins, resets)
        .enable(uart_config, peripheral_clock_freq)
        .expect("enabling uart for midi should succeed");
    midi_uart.enable_rx_interrupt();
    let (midi_reader, midi_writer) = midi_uart.split();
    (
        embedded_midi::MidiIn::new(midi_reader),
        embedded_midi::MidiOut::new(midi_writer),
    )
}

fn new_display(
    i2c: I2C1,
    sda_pin: DisplaySdaPin,
    scl_pin: DisplaySclPin,
    resets: &mut RESETS,
    peripheral_clock: &PeripheralClock,
) -> Display {
    let i2c_bus = I2C::i2c1(i2c, sda_pin, scl_pin, 1.MHz(), resets, peripheral_clock);

    let mut display = Ssd1306::new(
        I2CDisplayInterface::new_alternate_address(i2c_bus),
        DisplaySize128x64,
        DisplayRotation::Rotate0,
    )
    .into_buffered_graphics_mode();

    display.init().expect("display.init() should succeed");

    display
}
