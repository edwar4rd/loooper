use tokio::time;
use wiringpi::pin::Value;

pub async fn blink(current_millibeat: std::sync::Arc<std::sync::atomic::AtomicU32>) {
    let pi = wiringpi::setup();
    let pin_1 = pi.output_pin(15);
    let _pin_2 = pi.output_pin(16);

    let interval = time::Duration::from_millis(1);
    let mut current_state = Value::Low;

    loop {
        if current_millibeat.load(std::sync::atomic::Ordering::Relaxed) % 1000 < 250 {
            if current_state == Value::Low {
                current_state = Value::High;
                pin_1.digital_write(Value::High);
            }
        } else if current_state == Value::High {
            current_state = Value::Low;
            pin_1.digital_write(Value::Low);
        }
        time::sleep(interval).await;
    }
}
