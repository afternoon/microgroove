# microgroove

Firmware for a MIDI sequence generator.

This application is written in Rust using the [RTIC](https://rtic.rs) framework. It runs on a 
Raspberry Pi Pico (though could be modified to support other Cortex-M microcontrollers). The 
hardware has 6 rotary encoders, 3 buttons, an SSD1306 OLED display and 
[MIDI in and out circuits](https://diyelectromusic.wordpress.com/2021/02/15/midi-in-for-3-3v-microcontrollers/)
connected.

Microgroove has 8 tracks. Each track has a sequence which is transformed by a rhythm machine and a
melody machine. You can switch machines as you play, and control them with the encoders.
Machines can do things like generate random melodies or Euclidean rhythms. A few exist and, if
you're familiar with Rust, it's straightforward to add your own. Microgroove also has swing, 
quantization to different scales, and a set of groove parameters, similar to Ableton Live's 
groove pool.

If you're interested to build a Microgroove (and you should, its a lot of fun),
contact me at [ben@ben2.com](mailto:ben@ben2.com).
