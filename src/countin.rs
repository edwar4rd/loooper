use color_eyre::Result;
use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyEventKind};
use futures::{FutureExt, StreamExt};
use ratatui::{
    Frame,
    buffer::Buffer,
    layout::Rect,
    style::Stylize,
    style::{Color, Styled},
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget},
};

use crate::audio::AudioState;

#[derive(Debug)]
pub struct CountInState {
    /// The beats per minute (BPM).
    pub mbpm: u32,
    /// Whether to exit the application.
    pub exit: bool,
    /// Whether to enter the prepare phase.
    pub next_phase: bool,
    /// The event stream for receiving terminal events.
    pub event_stream: EventStream,
    /// The audio state.
    pub audio_state: AudioState,
}

impl CountInState {
    pub async fn handle_events(&mut self) -> Result<()> {
        let event = self.event_stream.next().fuse();
        tokio::select! {
            maybe_event = event => {
                if let Some(event) = maybe_event {
                    match event? {
                        Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                            self.handle_key_event(key_event)
                        }
                        _ => {}
                    }
                }
            }
            maybe_started = self.audio_state.started_rolling.recv() => {
                if let Some(()) = maybe_started {
                    self.transititon();
                }
            }
        }
        Ok(())
    }

    pub fn phase_changing(&self) -> bool {
        self.next_phase
    }

    pub fn exiting(&self) -> bool {
        self.exit
    }

    pub fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    pub fn from_prepare_state(prepare_state: crate::PrepareState) -> Self {
        CountInState {
            mbpm: prepare_state.mbpm,
            exit: false,
            next_phase: false,
            event_stream: prepare_state.event_stream,
            audio_state: prepare_state.audio_state,
        }
    }
}

impl CountInState {
    fn exit(&mut self) {
        self.exit = true;
    }

    fn transititon(&mut self) {
        self.next_phase = true;
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Char(' ') => self.transititon(),
            _ => {}
        }
    }
}

impl Widget for &CountInState {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Line::from(vec![
            " L".bold(),
            "O".set_style(Color::Rgb(26, 153, 136)).bold().italic(),
            "O".set_style(Color::Rgb(17, 85, 204)).bold().italic(),
            "O".set_style(Color::Rgb(180, 95, 6)).bold().italic(),
            "PER ".bold(),
            "(countin) ".italic(),
        ]);
        let instructions = Line::from(vec![" Quit ".into(), "<Q> ".blue().bold()]);
        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::THICK);

        let bpm = self.mbpm as f64 / 1000.;
        let counter_text = Text::from(vec![Line::from(vec![
            "BPM: ".into(),
            bpm.to_string().yellow(),
        ])]);

        Paragraph::new(counter_text)
            .centered()
            .block(block)
            .render(area, buf);
    }
}
