use tokio::time;
use wiringpi::pin::Value;

pub async fn blink(_current_millibeat: std::sync::Arc<std::sync::atomic::AtomicU32>) {
    let pi = wiringpi::setup();
    let pin_1 = pi.output_pin(15);
    let pin_2 = pi.output_pin(16);

    let interval = time::Duration::from_millis(500);

    loop {
        pin_1.digital_write(Value::High);
        time::sleep(interval).await;
        pin_2.digital_write(Value::High);
        time::sleep(interval).await;
        pin_2.digital_write(Value::Low);
        time::sleep(interval).await;
        pin_1.digital_write(Value::Low);
        time::sleep(interval).await;
    }
}
