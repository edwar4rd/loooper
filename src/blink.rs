use color_eyre::Result;
use std::{thread, time};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BitOrder {
    LSBFirst,
    MSBFirst,
}

fn shift_out(
    clock_pin: &mut rppal::gpio::OutputPin,
    data_pin: &mut rppal::gpio::OutputPin,
    order: BitOrder,
    data: u8,
) {
    for i in 0..8 {
        if order == BitOrder::MSBFirst {
            if data & (1 << i) != 0 {
                data_pin.set_high();
            } else {
                data_pin.set_low();
            }
        } else if data & (1 << (7 - i)) != 0 {
            data_pin.set_high();
        } else {
            data_pin.set_low();
        }
        clock_pin.set_high();
        clock_pin.set_low();
    }
}

fn display_image(
    data_pin: &mut rppal::gpio::OutputPin,
    latch_pin: &mut rppal::gpio::OutputPin,
    clock_pin: &mut rppal::gpio::OutputPin,
    interval: time::Duration,
    image: u64,
) {
    // Send each byte of the image
    for i in 0..8 {
        latch_pin.set_low();
        let b = 1 << i;
        let a = (image >> (i * 8)) as u8;
        shift_out(clock_pin, data_pin, BitOrder::LSBFirst, !b);
        shift_out(clock_pin, data_pin, BitOrder::MSBFirst, a);
        thread::sleep(interval);
        latch_pin.set_high();
    }
}

pub fn blink(
    current_millibeat: std::sync::Arc<std::sync::atomic::AtomicU32>,
    mut shutdown: tokio::sync::oneshot::Receiver<()>,
) -> Result<()> {
    let gpio = rppal::gpio::Gpio::new()?;
    let mut data_pin = gpio.get(10)?.into_output_low();
    let mut latch_pin = gpio.get(8)?.into_output_low();
    let mut clock_pin = gpio.get(11)?.into_output_low();

    let interval = time::Duration::from_micros(100);
    let mut last_beat = 0;

    const IMAGE_COUNT: usize = 10;
    const IMAGES: [u64; IMAGE_COUNT] = [
        0x3c66666e76663c00,
        0x7e1818181c181800,
        0x7e060c3060663c00,
        0x3c66603860663c00,
        0x30307e3234383000,
        0x3c6660603e067e00,
        0x3c66663e06663c00,
        0x1818183030667e00,
        0x3c66663c66663c00,
        0x3c66607c66663c00,
    ];
    let mut current_image = 0;

    latch_pin.set_low();
    shift_out(&mut clock_pin, &mut data_pin, BitOrder::LSBFirst, 255);
    shift_out(&mut clock_pin, &mut data_pin, BitOrder::LSBFirst, 0);
    latch_pin.set_high();

    loop {
        display_image(
            &mut data_pin,
            &mut latch_pin,
            &mut clock_pin,
            interval,
            IMAGES[current_image],
        );

        let current_beat = current_millibeat.load(std::sync::atomic::Ordering::Relaxed) / 1000;
        if current_beat != last_beat {
            // Switch to the next image
            current_image = current_beat as usize % IMAGE_COUNT;
            last_beat = current_beat;
        }

        if shutdown.try_recv().is_ok() {
            // If shutdown is requested, break the loop
            break;
        }
    }

    // Ensure the latch is low before exiting
    latch_pin.set_low();
    shift_out(&mut clock_pin, &mut data_pin, BitOrder::LSBFirst, 255);
    shift_out(&mut clock_pin, &mut data_pin, BitOrder::LSBFirst, 0);
    latch_pin.set_high();
    latch_pin.set_low();

    Ok(())
}
