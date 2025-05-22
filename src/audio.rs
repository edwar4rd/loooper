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

    let mut audio_clock: u128 = 0;
    let enabled_clone = enabled.clone();
    let mut last_enabled = false;
    let mbpm_clone = mbpm.clone();
    // let tx_clone = tx.clone();
    let process_callback = move |client: &jack::Client, ps: &jack::ProcessScope| {
        let sample_rate = client.sample_rate();
        let in_port = in_port.as_slice(ps);
        let out_port = out_port.as_mut_slice(ps);

        if !enabled_clone.load(std::sync::atomic::Ordering::Relaxed) {
            out_port.fill(0.0);
            return jack::Control::Continue;
        }

        if !last_enabled {
            audio_clock = 0;
        }
        last_enabled = true;

        let mbpm = mbpm_clone.load(std::sync::atomic::Ordering::Relaxed);
        let spb = 60.0 / mbpm as f32 * 1000.0;

        for (in_sample, out_sample) in in_port.iter().zip(out_port.iter_mut()) {
            let vol = if ((audio_clock as f32 / sample_rate as f32) / spb % 1.) < 0.125 {
                (1f32).min(((audio_clock as f32 / sample_rate as f32) / spb % 1.) * 100.)
            } else {
                (0f32).max(
                    1. - (((audio_clock as f32 / sample_rate as f32) / spb % 1.) - 0.125) * 10.,
                )
            };
            let wave = (audio_clock as f32 * 523.25 * 2.0 * std::f32::consts::PI
                / sample_rate as f32)
                .sin()
                * 0.2;
            let amp = vol * wave;

            *out_sample = amp + in_sample;

            audio_clock += 1;
        }
        jack::Control::Continue
    };
    let process = jack::contrib::ClosureProcessHandler::new(process_callback);

    let tx_clone = tx.clone();
    let active_client = client.activate_async(Notifications { tx: tx_clone }, process)?;

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
pub struct Notifications {
    pub tx: mpsc::UnboundedSender<String>,
}

impl jack::NotificationHandler for Notifications {
    fn thread_init(&self, _: &jack::Client) {
        let _ = self.tx.send("JACK: thread init".to_string());
    }

    /// Not much we can do here, see https://man7.org/linux/man-pages/man7/signal-safety.7.html.
    unsafe fn shutdown(&mut self, _: jack::ClientStatus, _: &str) {}

    fn freewheel(&mut self, _: &jack::Client, is_enabled: bool) {
        let _ = self.tx.send(format!(
            "JACK: freewheel mode is {}",
            if is_enabled { "on" } else { "off" }
        ));
    }

    fn sample_rate(&mut self, _: &jack::Client, srate: jack::Frames) -> jack::Control {
        let _ = self
            .tx
            .send(format!("JACK: sample rate changed to {srate}"));
        jack::Control::Continue
    }

    fn client_registration(&mut self, _: &jack::Client, name: &str, is_reg: bool) {
        let _ = self.tx.send(format!(
            "JACK: {} client with name \"{}\"",
            if is_reg { "registered" } else { "unregistered" },
            name
        ));
    }

    fn port_registration(&mut self, _: &jack::Client, port_id: jack::PortId, is_reg: bool) {
        let _ = self.tx.send(format!(
            "JACK: {} port with id {}",
            if is_reg { "registered" } else { "unregistered" },
            port_id,
        ));
    }

    fn port_rename(
        &mut self,
        _: &jack::Client,
        port_id: jack::PortId,
        old_name: &str,
        new_name: &str,
    ) -> jack::Control {
        let _ = self.tx.send(format!(
            "JACK: port with id {port_id} renamed from {old_name} to {new_name}",
        ));
        jack::Control::Continue
    }

    fn ports_connected(
        &mut self,
        _: &jack::Client,
        port_id_a: jack::PortId,
        port_id_b: jack::PortId,
        are_connected: bool,
    ) {
        let _ = self.tx.send(format!(
            "JACK: ports with id {} and {} are {}",
            port_id_a,
            port_id_b,
            if are_connected {
                "connected"
            } else {
                "disconnected"
            }
        ));
    }

    fn graph_reorder(&mut self, _: &jack::Client) -> jack::Control {
        let _ = self.tx.send("JACK: graph reordered".to_string());
        jack::Control::Continue
    }

    fn xrun(&mut self, _: &jack::Client) -> jack::Control {
        let _ = self.tx.send("JACK: xrun occurred".to_string());
        jack::Control::Continue
    }
}
