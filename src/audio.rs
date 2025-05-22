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
    let countin = Arc::new(AtomicBool::new(false));
    let countin_length = Arc::new(AtomicU32::new(0));
    let mbpm = Arc::new(AtomicU32::new(120));
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let loop_length = (0..8).map(|_| Arc::from(AtomicU32::new(4))).collect();
    let loop_starting = (0..8).map(|_| Arc::from(AtomicBool::new(false))).collect();
    let loop_layering = (0..8).map(|_| Arc::from(AtomicBool::new(false))).collect();
    let loop_playing = (0..8).map(|_| Arc::from(AtomicBool::new(false))).collect();

    let mut audio_clock: u64 = 0; // using u32 should panic in about a day
    let enabled_clone = enabled.clone();
    let mut last_enabled = false;
    let countin_clone = countin.clone();
    let mut countin_started = false;
    let countin_length_clone = countin_length.clone();
    let mut countin_left = 0;
    let mbpm_clone = mbpm.clone();
    let mut phase = 0.0;
    let mut adsr = crate::adsr::ADSR::new(0.01, 0.1, 0.2, 0.02);
    let mut click_freq = 523.25 / 2.0;
    let mut last_beat_pos = 0.999;
    // let tx_clone = tx.clone();
    let process_callback = move |client: &jack::Client, ps: &jack::ProcessScope| {
        let sample_rate = client.sample_rate() as u64;
        let in_port = in_port.as_slice(ps);
        let out_port = out_port.as_mut_slice(ps);

        if !enabled_clone.load(std::sync::atomic::Ordering::Relaxed) {
            out_port.fill(0.0);
            return jack::Control::Continue;
        }

        if !last_enabled {
            // We just got enabled
            audio_clock = 0;
            click_freq = 523.25 / 2.0;
        }
        last_enabled = true;

        let mbpm = mbpm_clone.load(std::sync::atomic::Ordering::Relaxed);
        let mspb = (60.0 / mbpm as f32 * 1000.0 * 1000.0) as u64;
        if !countin_started && countin_clone.load(std::sync::atomic::Ordering::Relaxed) {
            countin_clone.store(false, std::sync::atomic::Ordering::Relaxed);
            countin_left = countin_length_clone.load(std::sync::atomic::Ordering::Relaxed);
            countin_started = true;
        }

        for (in_sample, out_sample) in in_port.iter().zip(out_port.iter_mut()) {
            // Where we are inside a beat (0.0 - 1.0)
            let beat_pos = (audio_clock % (sample_rate * mspb / 1000)) as f32
                / ((sample_rate * mspb / 1000) as f32);

            // Set the sample to the input sample
            *out_sample = *in_sample;

            // We entered a new beat
            if beat_pos < last_beat_pos {
                // Reset the adsr for metronome
                adsr.reset();
                if countin_left == 0 {
                    countin_started = false;
                }
                click_freq = if countin_started {
                    countin_left -= 1;
                    if countin_left % 4 == 3 {
                        523.25
                    } else {
                        523.25 / 2.0
                    }
                } else {
                    440.0
                }
            }
            last_beat_pos = beat_pos;

            {
                // Set the adsr to release state after half a beat
                if beat_pos > 0.5 {
                    adsr.release();
                }
                let vol = adsr.forward(1.0 / (sample_rate as f32));
                phase += 1.0 / (sample_rate as f32 / click_freq) * 2.0 * std::f32::consts::PI;
                if phase > 2.0 * std::f32::consts::PI {
                    phase -= 2.0 * std::f32::consts::PI;
                }
                let wave = phase.sin() * 0.2;
                let amp = vol * wave;
                *out_sample += amp;
            }

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
        countin,
        countin_length,
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
    pub countin: Arc<AtomicBool>,                // Main -> Audio
    pub countin_length: Arc<AtomicU32>,          // Main -> Audio
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
