# Microgroove

8-track open-source hardware MIDI sequence generator.

- Machines offer different ways to generate sequences: random melodies, Euclidean rhythms, rhythms 
  from Mutable Instruments' Grids.
- Tweak parameters to explore new ideas, or to perform live.
- Quantize melodies to scales.
- Add swing and groove.
- Create interplay between sequences, like call-and-response, or ABAC structures.

## Why?

The modular world has some amazing tools to create interesting sequences: Turing Machine, Mutable
Instruments Grids, Mimetic Digitalis, Knight's Gallop, and so many more. The software ecosystem does
too, plugins like Stepic, modules in VCV Rack, and so many Max for Live experiments. In the desktop
MIDI hardware world, we're short on options (kudos to the Torso T1 for changing that), and that's
where I like to jam. I built Microgroove to bring some interesting sequencing ideas into a desktop
MIDI setup.

Microgroove is also a platform for experimentation. Trying new sequencing ideas is fast: create a
software change, flash it to the device next to you and start playing. Open sourcing the entire
device means anyone can extend or adapt it.

## Caution

Microgroove is a DIY electronic device that connects to other hardware. Please be careful if you're
working on the electronics yourself, or connecting to expensive equipment. I can't guarantee that
Microgroove won't break your gear.

## Quickstart

Connect at least one instrument to MIDI out, for example, a synth or a drum machine.

Connect a device with a sequencer to MIDI in. Microgroove doesn't have its own clock or transport
controls. It expects another device to be the master clock.

Microgroove's Track 1 is set to MIDI channel 1 by default. Set one of your instruments to listen to 
this channel, or change it from the track page.

Press play on your master sequencer. Microgroove will start playing a
randomly-generated 8-step sequence on MIDI channel 1.

Microgroove's MIDI out provides soft MIDI thru. Any MIDI notes coming from your
master sequencer will also be sent to your instruments.

### Tweak

Microgroove's philosophy is that generating a sequence and tweaking it is a
great way to create ideas. Use `[ENCODER1]` to `[ENCODER6]` to change
parameters. Use the `[TRACK]`, `[RHYTHM]` and `[MELODY]` buttons to change
between parameter pages. Press `[TRACK]` to cycle between the Track and
Sequence pages, `[RHYTHM]` to cycle between Rhythm and Groove pages, `[MELODY]`
for Melody and Harmony pages.

Each page lets you control an aspect of the current track, or the overall sequence.

- Track: Change rhythm and melody machines, length, time division and MIDI
  channel for the current track. Use `[ENCODER3]` to switch between tracks.
- Sequence: Set swing for all tracks (MPC format).
- Rhythm: Parameters for the selected rhythm machine.
- Groove: Set a part for this track, masking areas of the pattern.
- Melody: Parameters for the selected melody machine.
- Harmony: Quantize the melody to scale and key.

Choose rhythm and melody machines for each track, both are random by default.

To switch tracks, press `[TRACK]` to go to the Track page and choose a track
with `[ENCODER3]`. Tracks 2-8 are disabled by default. Choose a MIDI channel to
enable them.

Parts allow you to set up multiple tracks to play together in structures like
call-and-response or ABAC. Try setting Track 1 to `CALL` and Track 2 to
`RESP`, with all other parameters the same.

## Hardware

Microgroove is a simple device based around the Raspberry Pi Pico microcontroller. Building your own
should be straightforward (I knew nothing about electronics before I built it). The parts are fairly
standard and easy to get hold of from a few different electronics vendors, for example Pi Hut in the
UK or Adafruit in the US, and many more. You can build the device on a breadboard, or solder to
something like a [protoboard](https://www.adafruit.com/product/4785) and mount inside a laser-cut
case.

![Early Microgroove build on a breadboard](https://github.com/afternoon/microgroove/blob/main/hardware/microgroove-circuit-breadboard-photo.jpg)

### Components

| Component                                                                                                        | Quantity |
|----------------------------------------------------------------------------------------------------------------- | -------- |
| [Raspberry Pi Pico](https://www.raspberrypi.com/products/raspberry-pi-pico/)                                     | 1        |
| [Adafruit 128x64 1.3" Monochrome OLED](https://www.adafruit.com/product/938)                                     | 1        |
| [Adafruit NeoKey 1x4](https://www.adafruit.com/product/4980)                                                     | 1        |
| [Cherry MX-compatible key](https://thepihut.com/products/kailh-mechanical-key-switches-clicky-white-10-pack)     | 4        |
| [Cherry MX keycap](https://thepihut.com/products/relegendable-plastic-keycaps-for-mx-compatible-switches-5-pack) | 4        |
| [PEC11R rotary encoders](https://www.digikey.co.uk/en/products/detail/bourns-inc/PEC11R-4215F-S0024/4499665)     | 6        |
| [TRS minijacks](https://thepihut.com/products/breadboard-friendly-3-5mm-stereo-headphone-jack)                   | 2        |
| [H11L1 optoisolator](https://www.digikey.co.uk/en/products/detail/onsemi/H11L1TVM/401266)                        | 1        |
| [1N914 diode](https://www.digikey.co.uk/en/products/detail/onsemi/1N914/978749)                                  | 1        |
| 470Ω resistor                                                                                                    | 1        |
| 220Ω resistor                                                                                                    | 1        |
| 33Ω resistor                                                                                                     | 1        |
| 10Ω resistor                                                                                                     | 1        |
| Breadboard/protoboard                                                                                            | 2        |

### Build

This diagram shows the breadboard layout for Microgroove.

![Microgroove components shown on a breadboard](https://github.com/afternoon/microgroove/blob/main/hardware/microgroove-circuit-breadboard.png)

(If you want to build on a breadboard, you can get the Pico H, which has headers soldered on and
slots right on.)

See the [Fritzing
file](https://github.com/afternoon/microgroove/blob/main/hardware/microgroove-circuit.fzz) to view
the components and their connections. There is also a schematic view, but it is currently a mess.

The OLED display, NeoKey and encoders connect directly to pins on the Pico.

The MIDI section is also fairly simple. They are based on diyelectromusic's
[MIDI in](https://diyelectromusic.wordpress.com/2021/02/15/midi-in-for-3-3v-microcontrollers/) and
[MIDI out](https://diyelectromusic.wordpress.com/2021/01/23/midi-micropython-and-the-raspberry-pi-pico/)
circuits (thanks Kevin!). You can use TRS minijacks like me, or classic MIDI DIN jacks. Either way,
check the pinouts for the components you purchase carefully. Wrong wiring here might damage your
gear.

The NeoKey is a new addition and isn't essential. It can be replaced with a few generic push
buttons. I'm still writing the code to support it. The benefits of the NeoKey are per-button RGB
LEDs and a PCB that will hold Cherry MX-style clicky keys, which are 👌.

### Case

The case is laser-cut, in my case from 3mm ply. You can find
the [design as an SVG file
here](https://github.com/afternoon/microgroove/blob/main/hardware/microgroove-case-lasercut.svg).

The SVG file was creating in Tinkercad. You can [access the model
here](https://www.tinkercad.com/things/e7vA3MJyz0E-microgroove-box). You can clone it and make
your own modifications.

When cut, the case pieces slot together, and the components screw or glue to the case with standoffs
and M2 or M3 screws.

If you don't have access to a laser cutter, you should be able to find cutting services online.

## Firmware

The Microgroove firmware is written in Rust using the [RTIC](https://rtic.rs)
real-time framework. RTIC is truly wonderful. It lets us write clean Rust code
which multitasks with timing accurate to a few microseconds.

The app loosely follows the MVC architecture. The `microgroove_sequencer` library crate implements
the model. In the `microgroove_app` binary crate, the `display` module implements the view, while
the`app` module implements the controller.

Conceptually Microgroove is inspired by Elektron's machines, which make it fast to create and
manipulate musical ideas (the UI borrows Elektron's pages + encoders paradigm), and from modular,
which allows different task-specific components to be composed into a system. Different `Machine`s
can be combined to change how the sequence is generated.

The Machine concept is somewhat inspired by modular, where different modules can generate the
rhythm or the melody, or process it.

The code is split across two crates to allow the model and logic code to be platform-independent,
and therefore testable.

### Data model

A set of `struct`s in the `microgroove_sequencer` crate implement the data model.

- The top-level object is an instance of `Sequencer`.
- A `Sequencer` has many `Track`s. `Track` has a length, time division, MIDI channel, etc, and a
  `Sequence`.
- `Sequence` is a wrapper around a `Vec` of `Step`s, providing a grammar of methods for
  manipulating sequences in useful ways, e.g. setting the note numbers for steps from a `Vec` of
  `Note`s.
- `Step` has a `Note`, velocity, length and delay.
- `Note` is an `enum` of all MIDI note numbers.

Sequence generation is implemented by the `SequenceGenerator` struct. This is exposed to the RTIC
application separately from the data model, to allow the app to control how and when concrete
sequences are generated. A `SequenceGenerator` object has two `Machine`s. One to generate the rhythm
and a second to generate a melody. `Machine`s have an `apply` method which takes a `Sequence` and
transforms it. The process of generating a sequence is implemented as a pipeline in
`SequenceGenerator::generate`. A default `Sequence` is created and transformed by several
`Machine`s in order. The `Sequence` is then passed to a quantizer and to logic which applies parts -
removing steps from parts of the sequence.

### Building the firmware

If you haven't already, [install Rust](https://www.rust-lang.org/tools/install). If you aren't yet
familiar with Rust, I recommend reading the [Rust Book](https://doc.rust-lang.org/book/) to learn
the language.

Microgroove requires Rust nightly (it uses the
[linked_list_allocator](https://crates.io/crates/linked_list_allocator) crate, which requires the
`AllocRef` trait only in the nightly API). You'll also need to install the `thumbv6m-none-eabi`
target, which allows compilation for the Pi Pico's ARM Cortex-M0+ CPU, and
[cargo-embed](https://crates.io/crates/cargo-embed) which extends `cargo` with an `embed` command to
flash binaries to embedded devices.

```
$ rustup toolchain install nightly
$ rustup target add thumbv6m-none-eabi
$ cargo install cargo-embed
```

You can check your setup by running the `microgroove_sequencer` crate's unit tests.

```
$ cd firmware/microgroove_sequencer
$ cargo test
```

Connect the Pi Pico to your computer with USB and use `cargo-embed` to flash the app to your device.

```
$ cd ../microgroove_app
$ cargo embed
```

Your Microgroove should now be ready to play!

### Debugging

Serial output will be displayed on the console. See the
[cargo-embed](https://crates.io/crates/cargo-embed) docs for information on how to run GDB.

You can also use `probe-run` to flash binaries, but this requires a debug probe (which can be a 2nd
Pi Pico).

## Get in touch

Microgroove is still young and evolving fast. I'm be really interested to help out if you would like to build a device or contribute. I'd love to get your feedback on the process and also on how the device is to play, whether it’s fun, if you find the sequences in generates useful, what would make it more useful, and so on.

If you have feedback, ideas or questions, 
head over to the [discussions section](https://github.com/afternoon/microgroove/discussions)
or contact me at [ben@ben2.com](mailto:ben@ben2.com).
