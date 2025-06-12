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

use crate::{audio::AudioState, loops::LoopState};

#[derive(Debug)]
pub struct PrepareState {
    /// The beats per minute (BPM).
    pub mbpm: u32,
    /// Whether to exit the application.
    pub exit: bool,
    /// Whether to enter the prepare phase.
    pub next_phase: bool,
    /// The list of loops.
    pub loops: Vec<LoopState>,
    /// The event stream for receiving terminal events.
    pub event_stream: EventStream,
    /// The audio state.
    pub audio_state: AudioState,
    /// The index of the currently selected drum.
    pub drum_index: u32,
}

impl PrepareState {
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

    pub fn from_setup_state(setup_state: crate::SetUpState) -> Self {
        PrepareState {
            mbpm: setup_state.mbpm,
            exit: false,
            next_phase: false,
            loops: setup_state.loops,
            event_stream: setup_state.event_stream,
            audio_state: setup_state.audio_state,
            drum_index: setup_state.drum_index,
        }
    }
}

impl PrepareState {
    fn exit(&mut self) {
        self.exit = true;
    }

    fn transititon(&mut self) {
        self.next_phase = true;
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Char(' ') => self.start_countin(),
            _ => {}
        }
    }

    fn start_countin(&mut self) {
        for (index, loop_state) in self.loops.iter().enumerate() {
            self.audio_state.loop_length[index]
                .store(loop_state.beat_count, std::sync::atomic::Ordering::Relaxed);
            self.audio_state.loop_starting[index]
                .store(loop_state.starting, std::sync::atomic::Ordering::Relaxed);
            self.audio_state.loop_layering[index]
                .store(loop_state.layering, std::sync::atomic::Ordering::Relaxed);
        }
        for index in self.loops.len()..8 {
            self.audio_state.loop_starting[index]
                .store(false, std::sync::atomic::Ordering::Relaxed);
        }
        self.audio_state
            .countin_length
            .store(8, std::sync::atomic::Ordering::Relaxed);
        self.audio_state
            .countin
            .store(true, std::sync::atomic::Ordering::Relaxed);
        self.transititon();
    }
}

impl Widget for &PrepareState {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Line::from(vec![
            " L".bold(),
            "O".set_style(Color::Rgb(26, 153, 136)).bold().italic(),
            "O".set_style(Color::Rgb(17, 85, 204)).bold().italic(),
            "O".set_style(Color::Rgb(180, 95, 6)).bold().italic(),
            "PER ".bold(),
            "(prepare) ".italic(),
        ]);
        let instructions = Line::from(vec![
            " Start Count-in ".into(),
            "<Space>".blue().bold(),
            " Quit ".into(),
            "<Q> ".blue().bold(),
        ]);
        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::THICK);

        let bpm = self.mbpm as f64 / 1000.;
        let mut texts = Vec::new();
        let counter_line = Line::from(vec!["BPM: ".into(), bpm.to_string().yellow()]);
        texts.push(counter_line);

        let drum_line = Line::from(vec![
            "Drum: ".into(),
            format!("{}", self.drum_index).yellow(), // +1 if you want 1–5 instead of 0–4
        ]);
        texts.push(drum_line);

        for (i, loop_state) in self.loops.iter().enumerate() {
            let loop_text = Line::from(vec![
                if loop_state.starting {
                    "🟢".green()
                } else {
                    "🟥".red()
                },
                format!(" Loop {}: ", i + 1).into(),
                format!("{} beats, ", loop_state.beat_count).yellow(),
                if loop_state.layering {
                    "layering".green()
                } else {
                    "overwriting".red()
                },
            ]);
            texts.push(loop_text);
        }

        Paragraph::new(Text::from(texts))
            .centered()
            .block(block)
            .render(area, buf);
    }
}
