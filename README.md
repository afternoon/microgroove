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

## Hardware

![Early Microgroove build on a breadboard](https://github.com/afternoon/microgroove/blob/main/hardware/microgroove-circuit-breadboard-photo.jpg)

Microgroove is a simple device based around the Raspberry Pi Pico microcontroller.
Building your own should be straightforward (I didn't know anything about electronics before I built it).
The parts are fairly standard and easy to get hold of from a few different electronics vendors, for example Pi Hut in the UK, Adafruit in the US, and many more.

### Building a Microgroove

Here is a breadboard diagram from Fritzing (as PNG, and the original FZZ file). Fritzing has a schematic view as well, but I didn't tidy that up (yet) so it's a mess. The Fritzing file also gives you the BOM, although the jacks and encoders are different parts from the ones I've used, and have different pinouts.

![Microgroove components shown on a breadboard](https://github.com/afternoon/microgroove/blob/main/hardware/microgroove-circuit-breadboard.png)

As you can see, the MIDI parts are the only interesting parts really. The encoders, OLED screen and NeoKey are all connected straight to the Pi Pico, which is the MCU I'm using.

The NeoKey is a new addition and isn't essential. It can be replaced with a few generic push buttons. I'm still writing the code to add it, so the code on GitHub expects buttons. The benefits of the NeoKey are per-button LEDs and a PCB that will hold Cherry MX-style clicky keys.

These are the encoders I’m using: 
https://www.digikey.co.uk/en/products/detail/bourns-inc/PEC11R-4215F-S0024/4499665

I've also attached the SVG for the case laser cutting. This hopefully will give you some idea of what the end result should look like. I created this in Tinkercad. You can see the model here: https://www.tinkercad.com/things/e7vA3MJyz0E-microgroove-box

See the Fritzing file for the required components and how to connect them.

The circuit is pretty straightforward. I didn’t know any electronics before I built it.

[MIDI in and out circuits](https://diyelectromusic.wordpress.com/2021/02/15/midi-in-for-3-3v-microcontrollers/).

### Case

The case is laser cut, in my case from 3mm ply. If you have access to a laser cutter, you can find the [design as an SVG file here](https://github.com/afternoon/microgroove/blob/main/hardware/microgroove-case-lasercut.svg). If not, you should be able to find cutting services online.

When cut, the case pieces slot together, and the components screw or glue to the case with standoffs and M2 or M3 screws.

## Get in touch

Microgroove is still young and evolving fast. I'm be really interested to help out if you would like to build a device or contribute. I'd love to get your feedback on the process and also on how the device is to play, whether it’s fun, if you find the sequences in generates useful, what would make it more useful, and so on.

If you have feedback, ideas or questions, 
head over to the [discussions section](https://github.com/afternoon/microgroove/discussions)
or contact me at [ben@ben2.com](mailto:ben@ben2.com).
