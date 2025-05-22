use color_eyre::Result;
use jack::{Client, Control, PortFlags, ProcessScope};

pub fn audio_setup() -> Result<(
    jack::AsyncClient<
        Notifications,
        jack::contrib::ClosureProcessHandler<(), impl FnMut(&Client, &ProcessScope) -> Control>,
    >,
    AudioState,
)> {
    // TODO: Integrate logging with the gui thread
    jack::set_logger(jack::LoggerType::None);

    let (client, _status) = jack::Client::new("loooper", jack::ClientOptions::default())?;

    let in_port = client.register_port("loooper_in", jack::AudioIn::default())?;
    let mut out_port = client.register_port("loooper_out", jack::AudioOut::default())?;

    let enabled = Arc::new(AtomicBool::new(false));
    let mbpm = Arc::new(AtomicU32::new(120));
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let loop_length = (0..8).map(|_| Arc::from(AtomicU32::new(4))).collect();
    let loop_starting = (0..8).map(|_| Arc::from(AtomicBool::new(false))).collect();
    let loop_layering = (0..8).map(|_| Arc::from(AtomicBool::new(false))).collect();
    let loop_playing = (0..8).map(|_| Arc::from(AtomicBool::new(false))).collect();

    let process_callback = move |_: &jack::Client, ps: &jack::ProcessScope| {
        let in_port = in_port.as_slice(ps);
        let out_port = out_port.as_mut_slice(ps);

        out_port.clone_from_slice(in_port);
        jack::Control::Continue
    };
    let process = jack::contrib::ClosureProcessHandler::new(process_callback);

    let active_client = client.activate_async(Notifications, process)?;

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
        mbpm,
        errors: rx,
        loop_length,
        loop_starting,
        loop_layering,
        loop_playing,
    };
    Ok((active_client, state))
}

use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU32},
};
use tokio::sync::mpsc;

#[test]
fn test_host_device_setup() {
    let result = audio_setup();
    assert!(result.is_ok());
    let _ = result.unwrap();
}

#[derive(Debug)]
pub struct AudioState {
    pub enabled: Arc<AtomicBool>,                // Main -> Audio
    pub mbpm: Arc<AtomicU32>,                    // Main -> Audio
    pub errors: mpsc::UnboundedReceiver<String>, // Audio -> Main
    pub loop_length: Vec<Arc<AtomicU32>>,        // Main -> Audio
    pub loop_starting: Vec<Arc<AtomicBool>>,     // Main -> Audio
    pub loop_layering: Vec<Arc<AtomicBool>>,     // Main -> Audio
    pub loop_playing: Vec<Arc<AtomicBool>>,      // Audio -> Main
}

// Taken from https://github.com/RustAudio/rust-jack/blob/main/examples/playback_capture.rs
pub struct Notifications;

impl jack::NotificationHandler for Notifications {
    fn thread_init(&self, _: &jack::Client) {
        println!("JACK: thread init");
    }

    /// Not much we can do here, see https://man7.org/linux/man-pages/man7/signal-safety.7.html.
    unsafe fn shutdown(&mut self, _: jack::ClientStatus, _: &str) {}

    fn freewheel(&mut self, _: &jack::Client, is_enabled: bool) {
        println!(
            "JACK: freewheel mode is {}",
            if is_enabled { "on" } else { "off" }
        );
    }

    fn sample_rate(&mut self, _: &jack::Client, srate: jack::Frames) -> jack::Control {
        println!("JACK: sample rate changed to {srate}");
        jack::Control::Continue
    }

    fn client_registration(&mut self, _: &jack::Client, name: &str, is_reg: bool) {
        println!(
            "JACK: {} client with name \"{}\"",
            if is_reg { "registered" } else { "unregistered" },
            name
        );
    }

    fn port_registration(&mut self, _: &jack::Client, port_id: jack::PortId, is_reg: bool) {
        println!(
            "JACK: {} port with id {}",
            if is_reg { "registered" } else { "unregistered" },
            port_id
        );
    }

    fn port_rename(
        &mut self,
        _: &jack::Client,
        port_id: jack::PortId,
        old_name: &str,
        new_name: &str,
    ) -> jack::Control {
        println!("JACK: port with id {port_id} renamed from {old_name} to {new_name}",);
        jack::Control::Continue
    }

    fn ports_connected(
        &mut self,
        _: &jack::Client,
        port_id_a: jack::PortId,
        port_id_b: jack::PortId,
        are_connected: bool,
    ) {
        println!(
            "JACK: ports with id {} and {} are {}",
            port_id_a,
            port_id_b,
            if are_connected {
                "connected"
            } else {
                "disconnected"
            }
        );
    }

    fn graph_reorder(&mut self, _: &jack::Client) -> jack::Control {
        println!("JACK: graph reordered");
        jack::Control::Continue
    }

    fn xrun(&mut self, _: &jack::Client) -> jack::Control {
        println!("JACK: xrun occurred");
        jack::Control::Continue
    }
}
