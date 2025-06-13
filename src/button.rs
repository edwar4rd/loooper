use std::{thread::sleep, time::Duration};
use wiringpi::pin::Value;

const BUTTON_PINS: [u16; 13] = [23, 22, 21, 3, 2, 0, 7, 27, 26, 15, 16, 5, 6];

pub fn button(
    pad_tx: tokio::sync::mpsc::UnboundedSender<usize>,
    mut shutdown: tokio::sync::oneshot::Receiver<()>,
) {
    let pi = wiringpi::setup();
    let mut pins = Vec::new();
    let mut last_states = vec![false; BUTTON_PINS.len()];
    for pin in BUTTON_PINS {
        let pin = pi.input_pin(pin);
        pin.pull_up_dn_control(wiringpi::pin::Pull::Up);
        pins.push(pin);
    }

    let interval = Duration::from_micros(100);

    loop {
        for ((button_id, pin), last_state) in
            pins.iter_mut().enumerate().zip(last_states.iter_mut())
        {
            let pin_state = pin.digital_read();
            if pin_state == Value::Low && !(*last_state) {
                *last_state = true;
                if button_id > 8 {
                    let pad_id = button_id - 9;
                    let _ = pad_tx.send(pad_id);
                }
            } else {
                *last_state = false;
            }
        }

        if shutdown.try_recv().is_ok() {
            break;
        }

        sleep(interval);
    }
}
