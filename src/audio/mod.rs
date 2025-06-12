use color_eyre::Result;
use jack::PortFlags;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU32},
};
use tokio::sync::mpsc;

#[derive(Debug)]
pub struct AudioState {
    pub enabled: Arc<AtomicBool>,                     // Main -> Audio
    pub countin: Arc<AtomicBool>,                     // Main -> Audio
    pub countin_length: Arc<AtomicU32>,               // Main -> Audio
    pub started_rolling: mpsc::UnboundedReceiver<()>, // Audio -> Main
    pub mbpm: Arc<AtomicU32>,                         // Main -> Audio
    pub messages: mpsc::UnboundedReceiver<String>,    // Audio -> Main
    pub loop_length: Vec<Arc<AtomicU32>>,             // Main -> Audio
    pub loop_starting: Vec<Arc<AtomicBool>>,          // Main -> Audio
    pub loop_layering: Vec<Arc<AtomicBool>>,          // Main -> Audio
    pub loop_playing: Vec<Arc<AtomicBool>>,           // Audio -> Main
    pub loop_recording: Vec<Arc<AtomicBool>>,         // Audio -> Main
    pub current_millibeat: Arc<AtomicU32>,            // Audio -> Main
    pub pad_tx: mpsc::UnboundedSender<usize>,
}

mod adsr;
mod callback;
mod notifications;
mod oscillator;

pub fn audio_setup() -> Result<(
    jack::AsyncClient<impl jack::NotificationHandler, impl jack::ProcessHandler>,
    AudioState,
)> {
    // TODO: Integrate logging with the gui thread
    jack::set_logger(jack::LoggerType::None);

    let (client, _status) = jack::Client::new("loooper", jack::ClientOptions::default())?;

    let in_port = client.register_port("loooper_in", jack::AudioIn::default())?;
    let out_port = client.register_port("loooper_out", jack::AudioOut::default())?;

    let enabled = Arc::new(AtomicBool::new(false));
    let countin = Arc::new(AtomicBool::new(false));
    let countin_length = Arc::new(AtomicU32::new(0));
    let (rolling_tx, rolling_rx) = tokio::sync::mpsc::unbounded_channel();
    let mbpm = Arc::new(AtomicU32::new(120));
    let (message_tx, message_rx) = tokio::sync::mpsc::unbounded_channel();
    let loop_length: Vec<_> = (0..8).map(|_| Arc::from(AtomicU32::new(4))).collect();
    let loop_starting: Vec<_> = (0..8).map(|_| Arc::from(AtomicBool::new(false))).collect();
    let loop_layering: Vec<_> = (0..8).map(|_| Arc::from(AtomicBool::new(false))).collect();
    let loop_playing: Vec<_> = (0..8).map(|_| Arc::from(AtomicBool::new(false))).collect();
    let loop_recording: Vec<_> = (0..8).map(|_| Arc::from(AtomicBool::new(false))).collect();
    let current_millibeat = Arc::new(AtomicU32::new(0));
    let (pad_tx, pad_rx) = tokio::sync::mpsc::unbounded_channel::<usize>();

    let notification_handler = notifications::Notifications {
        tx: message_tx.clone(),
    };
    let callback_handler = callback::create_callback(callback::AudioCallbackSettings {
        sample_rate: client.sample_rate(),
        in_port,
        out_port,
        enabled: enabled.clone(),
        countin: countin.clone(),
        countin_length: countin_length.clone(),
        rolling_tx,
        mbpm: mbpm.clone(),
        loop_length: loop_length.clone(),
        loop_starting: loop_starting.clone(),
        loop_playing: loop_playing.clone(),
        loop_recording: loop_recording.clone(),
        current_millibeat: current_millibeat.clone(),
        pad_rx,
    });

    let active_client = client.activate_async(notification_handler, callback_handler)?;

    {
        let src_ports = active_client.as_client().ports(
            None,
            Some("32 bit float mono audio"),
            PortFlags::IS_OUTPUT.union(PortFlags::IS_PHYSICAL),
        );
        if let Some(port) = src_ports.first() {
            active_client
                .as_client()
                .connect_ports_by_name(port.as_str(), "loooper:loooper_in")
                .unwrap();
        }

        let dest_ports = active_client.as_client().ports(
            None,
            Some("32 bit float mono audio"),
            PortFlags::IS_INPUT.union(PortFlags::IS_PHYSICAL),
        );
        for port in &dest_ports {
            active_client
                .as_client()
                .connect_ports_by_name("loooper:loooper_out", port.as_str())
                .unwrap();
        }
    }

    let state = AudioState {
        enabled,
        countin,
        countin_length,
        started_rolling: rolling_rx,
        mbpm,
        messages: message_rx,
        loop_length,
        loop_starting,
        loop_layering,
        loop_playing,
        loop_recording,
        current_millibeat,
        pad_tx,
    };
    Ok((active_client, state))
}

#[test]
fn test_host_device_setup() {
    let result = audio_setup();
    assert!(result.is_ok());
    let _ = result.unwrap();
}
