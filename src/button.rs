use color_eyre::Result;
use std::time::Duration;

const BUTTON_PINS: [u8; 13] = [23, 22, 21, 3, 2, 0, 7, 27, 26, 15, 16, 5, 6];

pub fn button(
    pad_tx: tokio::sync::mpsc::UnboundedSender<usize>,
    button_tx: tokio::sync::mpsc::UnboundedSender<usize>,
    mut shutdown: tokio::sync::oneshot::Receiver<()>,
) -> Result<()> {
    let gpio = rppal::gpio::Gpio::new()?;
    let mut pins = Vec::new();
    for (button_id, pin) in BUTTON_PINS.iter().enumerate() {
        let mut pin = gpio.get(*pin)?.into_input_pullup();
        if (1..=8).contains(&button_id) {
            pin.set_interrupt(rppal::gpio::Trigger::Both, Some(Duration::from_millis(50)))?;
        } else {
            pin.set_interrupt(
                rppal::gpio::Trigger::FallingEdge,
                Some(Duration::from_millis(50)),
            )?;
        }
        pins.push(pin);
    }
    let polled_pins: Vec<&rppal::gpio::InputPin> = pins.iter().collect();

    let interval = Duration::from_micros(100);

    loop {
        if let Some((pin, _event)) = gpio.poll_interrupts(&polled_pins, false, Some(interval))? {
            let button_id = BUTTON_PINS.iter().position(|&p| p == pin.pin()).unwrap() + 1;
            if button_id > 8 {
                let pad_id = button_id - 9;
                let _ = pad_tx.send(pad_id);
            }
            let _ = button_tx.send(button_id);
        }

        if shutdown.try_recv().is_ok() {
            break;
        }
    }

    Ok(())
}
