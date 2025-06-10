use tokio::time;
use wiringpi::pin::{OutputPin, Value, WiringPi};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BitOrder {
    LSBFirst,
    MSBFirst,
}

async fn shift_out(
    clock_pin: &OutputPin<WiringPi>,
    data_pin: &OutputPin<WiringPi>,
    order: BitOrder,
    data: u8,
) {
    for i in 0..8 {
        if order == BitOrder::MSBFirst {
            if data & (1 << i) != 0 {
                data_pin.digital_write(Value::High);
            } else {
                data_pin.digital_write(Value::Low);
            }
        } else if data & (1 << (7 - i)) != 0 {
            data_pin.digital_write(Value::High);
        } else {
            data_pin.digital_write(Value::Low);
        }
        clock_pin.digital_write(Value::High);
        clock_pin.digital_write(Value::Low);
    }
}

async fn display_image(
    data_pin: &OutputPin<WiringPi>,
    latch_pin: &OutputPin<WiringPi>,
    clock_pin: &OutputPin<WiringPi>,
    interval: time::Duration,
    image: u64,
) {
    // Send each byte of the image
    for i in 0..8 {
        latch_pin.digital_write(Value::Low);
        let b = 1 << i;
        let a = (image >> (i * 8)) as u8;
        shift_out(clock_pin, data_pin, BitOrder::LSBFirst, !b).await;
        shift_out(clock_pin, data_pin, BitOrder::MSBFirst, a).await;
        time::sleep(interval).await;
        latch_pin.digital_write(Value::High);
    }
}

pub async fn blink(
    current_millibeat: std::sync::Arc<std::sync::atomic::AtomicU32>,
    mut shutdown: tokio::sync::oneshot::Receiver<()>,
) {
    let pi = wiringpi::setup();
    let data_pin = pi.output_pin(15);
    let latch_pin = pi.output_pin(16);
    let clock_pin = pi.output_pin(27);

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

    latch_pin.digital_write(Value::Low);
    shift_out(&clock_pin, &data_pin, BitOrder::LSBFirst, 255).await;
    shift_out(&clock_pin, &data_pin, BitOrder::LSBFirst, 0).await;
    latch_pin.digital_write(Value::High);

    loop {
        display_image(
            &data_pin,
            &latch_pin,
            &clock_pin,
            interval,
            IMAGES[current_image],
        )
        .await;

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
    latch_pin.digital_write(Value::Low);
    shift_out(&clock_pin, &data_pin, BitOrder::LSBFirst, 255).await;
    shift_out(&clock_pin, &data_pin, BitOrder::LSBFirst, 0).await;
    latch_pin.digital_write(Value::High);
    latch_pin.digital_write(Value::Low);
}
