use tokio::sync::mpsc;

// Taken from https://github.com/RustAudio/rust-jack/blob/main/examples/playback_capture.rs
pub struct Notifications {
    pub tx: mpsc::UnboundedSender<String>,
}

impl jack::NotificationHandler for Notifications {
    fn thread_init(&self, _: &jack::Client) {
        let _ = self.tx.send("JACK: thread init".to_string());
    }

    /// Not much we can do here, see <https://man7.org/linux/man-pages/man7/signal-safety.7.html>.
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
