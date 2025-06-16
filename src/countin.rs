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
pub struct CountInState {
    /// The beats per minute (BPM).
    pub mbpm: u32,
    /// Whether to exit the application.
    pub exit: bool,
    /// Whether to enter the prepare phase.
    pub next_phase: bool,
    /// The selected loop
    pub selected: usize,
    /// The list of loops.
    pub loops: Vec<LoopState>,
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
            selected: 0,
            loops: prepare_state.loops,
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

    fn select_next(&mut self) {
        if self.selected + 1 >= self.loops.len() {
            self.selected = 0;
        } else {
            self.selected += 1;
        }
    }

    fn select_priv(&mut self) {
        if self.selected == 0 {
            self.selected = self.loops.len() - 1;
        } else {
            self.selected -= 1;
        }
    }

    fn toggle_starting(&mut self) {
        self.audio_state.loop_starting[self.selected]
            .fetch_not(std::sync::atomic::Ordering::Relaxed);
    }

    fn mark_recording(&mut self) {
        // self.audio_state.loop_starting[self.selected].store(true, std::sync::atomic::Ordering::Relaxed);
        todo!("Wait what we do not have a recording marker???");
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Up => self.select_priv(),
            KeyCode::Down => self.select_next(),
            KeyCode::Char(' ') => self.toggle_starting(),
            KeyCode::Enter => self.mark_recording(),
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
        let instructions = Line::from(vec![
            " Toggle Starting ".into(),
            "<Space> ".blue().bold(),
            " Toggle Recording ".into(),
            "<Enter> ".blue().bold(),
            " Quit ".into(),
            "<Q> ".blue().bold(),
        ]);
        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::THICK);

        let bpm = self.mbpm as f64 / 1000.;
        let mut texts = vec![Line::from(vec!["BPM: ".into(), bpm.to_string().yellow()])];
        for (index, loop_state) in self.loops.iter().enumerate() {
            let loop_text = Line::from(vec![
                if self.selected == index {
                    ">> ".green()
                } else {
                    "".into()
                },
                if loop_state.wah {
                    "W".set_style(Color::Rgb(255, 0, 0)).bold()
                } else {
                    "W".set_style(Color::Rgb(128, 128, 128)).bold()
                },
                if loop_state.reverb {
                    "R".set_style(Color::Rgb(0, 255, 0)).bold()
                } else {
                    "R".set_style(Color::Rgb(128, 128, 128)).bold()
                },
                if loop_state.distortion {
                    "D".set_style(Color::Rgb(0, 0, 255)).bold()
                } else {
                    "D".set_style(Color::Rgb(128, 128, 128)).bold()
                },
                if self.audio_state.loop_starting[index].load(std::sync::atomic::Ordering::Relaxed)
                {
                    "🟢".green()
                } else {
                    "🟥".red()
                },
                format!(" Loop {}: ", index + 1).into(),
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
