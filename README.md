# LOOOPER

*A project for 2025 Spring Practical Digital Electronics course*

## Requirements

- [x] 3 or more separate loops
- [x] the ability to record and playback into the loops
- [x] beat indication / metronome (clicks)
- [ ] a straight forward (preferably physical) user interface

## Nice to haves

- [x] Effects (e.g. filters, distortions, reverbs)
- [ ] Playback in different speed
- [ ] Pitch shifting or correction
- [ ] More I/O
- [ ] *<span style="color:#F88;">R</span><span style="color:#8F8;">G</span><span style="color:#88F;">B</span> Lighting*

## Project Structure

- In the center of our project is a twenty-ish dollar Raspberry Pi Zero 2W, which connects to the audio interface and perform the main audio processing.
- For demo purposes, we currently employ my Focusrite Clarett+ 4pre for a custom built solution, which is connected to the Pi with its USB port.
  - We are currently trying to make a SparkFun WM8960 board work with the Pi.
- As of the user interface side, the Pi have 24 GPIO pins available. This can be adequate for our project if all we need are buttons and LEDs. If we require analog inputs (i.e. faders and/or pots) in the future, it's possible to connect a Arduino or a ESP32 to the Pi via I2C, SPI, or UART.
- In the software side, we currently uses the stock Raspberry Pi OS, but have plans to switch to Diet Pi if required.
