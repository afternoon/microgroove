# Microgroove

8-track open-source hardware MIDI sequence generator.

- Machines offer different ways to generate sequences: random melodies, Euclidean rhythms, rhythms 
  from Mutable Instruments' Grids.
- Tweak parameters to explore new ideas, or to perform live.
- Quantize melodies to scales.
- Add swing and groove.
- Create interplay between sequences, like call-and-response, or ABAC structures.

## Why?

The modular world has some amazing tools for harnessing randomness to create interesting sequences:
Turing Machine, Mutable Instruments Grids, Mimetic Digitalis, and so many more. The plugin ecosystem 
does too, but I like to play with desktop hardware boxes. I built Microgroove to bring some of the 
interesting sequencing ideas into a desktop MIDI setup.

Microgroove is also a platform for experimentation. Trying new sequencing ideas is fast: create a 
software change, flash it to the device next to you and start playing. Open sourcing the entire
device means anyone can extend or adapt it.

## Caution

Microgroove is a DIY electronic device that connects to other hardware. Please be careful if you're
working on the electronics yourself, or connecting to expensive equipment. I can't guarantee that
Microgroove won't break your gear.

## Quickstart

### Get connected

Connect at least one instrument to MIDI out, for example, a synth or a drum machine.

Connect a device with a sequencer to MIDI in. Microgroove doesn't have its own clock or transport
controls. It expects another device to be the master clock.

Microgroove's Track 1 is set to MIDI channel 1 by default. Set one of your instruments to listen to 
this channel, or change it from the track page.

### Let's sequence

Press/double-press buttons to change mode. Modes:

Track: Choose rhythm and melody machines, change sequence length, time division and MIDI channel,
switch between tracks.
Sequence: Set swing (MPC format).
Rhythm: Parameters for the selected rhythm machine.
Groove: Set a "part" for this track, which masks areas of the sequence.
Melody: Parameters for the selected melody machine.
Harmony: Quantize melody to scale and key.

Choose rhythm and melody machines for each track, both are random by default.

### Play!

Press play on your master sequencer.

### More stuff to try

To switch tracks, press [TRACK] to go to the track page, and choose a track with [ENCODER3]. Tracks 2-8 are disabled by default. Choose a MIDI channel to enable them.

Parts allow you to set up multiple tracks to play together in structures like call-and-response or ABAC.

Microgroove's MIDI out is soft MIDI thru. Messages are copied from MIDI in to MIDI out.

# Firmware

The Microgroove firmware is written in Rust using the [RTIC](https://rtic.rs) real-time framework.

## Architecture

TODO. RTIC, Sequencers, Tracks, Machines, Params, oh my!

Microgroove is heavily inspired by Elektron's machines, like the Digitakt or Octatrack. These
machines make it fast to create and manipulate something.

The Machine concept is somewhat inspired by modular, where different modules can generate the
rhythm or the melody, or process it.

## Building the firmware

- Set up the Rust embedded toolchain on your machine.
- Connect the Pico by USB and run `cargo run` to flash to the device.

## Debugging

Serial output will be displayed on the console via `defmt`.

It's possible to debug with GDB. Ask me about it sometime.

# Hardware

Microgroove is a simple device based around the Raspberry Pi Pico microcontroller.
Building your own should be straightforward (I didn't know anything about electronics before I built it).
The parts are fairly standard and easy to get hold of from a few different electronics vendors, for example Pi Hut in the UK, Adafruit in the US, and many more.

## Building a Microgroove

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

# Get in touch

Microgroove is still young and evolving fast. I'm be really interested to help out if you would like to build a device or contribute. I'd love to get your feedback on the process and also on how the device is to play, whether it’s fun, if you find the sequences in generates useful, what would make it more useful, and so on.

If you have feedback, ideas or questions, 
head over to the [discussions section](https://github.com/afternoon/microgroove/discussions)
or contact me at [ben@ben2.com](mailto:ben@ben2.com).
