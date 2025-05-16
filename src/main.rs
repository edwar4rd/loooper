use color_eyre::Result;
use crossterm::event::{self, Event};
use loooper::audio;
use ratatui::{DefaultTerminal, Frame};

pub struct State {
    bpm: f32,
    time_signature: (u32, u32),
    metronome: bool,
    measure: u32,
    sink: cpal::Stream,
    source: cpal::Stream,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let _ = audio::host_device_setup()?;
    let terminal = ratatui::init();
    let result = run(terminal);
    ratatui::restore();
    result
}

fn run(mut terminal: DefaultTerminal) -> Result<()> {
    loop {
        terminal.draw(render)?;
        if matches!(event::read()?, Event::Key(_)) {
            break Ok(());
        }
    }
}

fn render(frame: &mut Frame) {
    frame.render_widget("hello world", frame.area());
}
