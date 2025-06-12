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
pub struct RollingState {
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

impl RollingState {
    pub async fn handle_events(&mut self) -> Result<()> {
        let event = self.event_stream.next().fuse();
        let sleep = tokio::time::sleep(std::time::Duration::from_millis(50));
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
            },
            _ = sleep => {

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

    pub fn from_countin_state(countin_state: crate::CountInState) -> Self {
        RollingState {
            mbpm: countin_state.mbpm,
            exit: false,
            next_phase: false,
            selected: countin_state.selected,
            loops: countin_state.loops,
            event_stream: countin_state.event_stream,
            audio_state: countin_state.audio_state,
        }
    }
}

impl RollingState {
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
            KeyCode::Char('1') => {
                let _ = self.audio_state.pad_tx.send(0);
            }
            KeyCode::Char('2') => {
                let _ = self.audio_state.pad_tx.send(1);
            }
            KeyCode::Char('3') => {
                let _ = self.audio_state.pad_tx.send(2);
            }
            KeyCode::Char('q') => self.exit(),
            KeyCode::Esc => self.transititon(),
            KeyCode::Up => self.select_priv(),
            KeyCode::Down => self.select_next(),
            KeyCode::Char(' ') => self.toggle_starting(),
            KeyCode::Enter => self.mark_recording(),
            _ => {}
        }
    }
}

impl Widget for &RollingState {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Line::from(vec![
            " L".bold(),
            "O".set_style(Color::Rgb(26, 153, 136)).bold().italic(),
            "O".set_style(Color::Rgb(17, 85, 204)).bold().italic(),
            "O".set_style(Color::Rgb(180, 95, 6)).bold().italic(),
            "PER ".bold(),
            "(rolling) ".italic(),
        ]);
        let instructions = Line::from(vec![
            " Toggle Starting ".into(),
            "<Space> ".blue().bold(),
            " Toggle Recording ".into(),
            "<Enter> ".blue().bold(),
            " Reset Loooper ".into(),
            "<Esc>".blue().bold(),
            " Quit ".into(),
            "<Q> ".blue().bold(),
        ]);
        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::THICK);

        let bpm = self.mbpm as f64 / 1000.;
        let current_millibeat = self
            .audio_state
            .current_millibeat
            .load(std::sync::atomic::Ordering::Relaxed);
        let mut texts = vec![Line::from(vec![
            "BPM: ".into(),
            bpm.to_string().yellow(),
            format!(
                " Beat: {}.{}",
                current_millibeat / 1000,
                current_millibeat % 1000
            )
            .into(),
        ])];
        for (index, loop_state) in self.loops.iter().enumerate() {
            let loop_text = Line::from(vec![
                if self.selected == index {
                    ">> ".green()
                } else {
                    "".into()
                },
                if self.audio_state.loop_starting[index].load(std::sync::atomic::Ordering::Relaxed)
                {
                    "🟢".green()
                } else {
                    "🟥".red()
                },
                if self.audio_state.loop_playing[index].load(std::sync::atomic::Ordering::Relaxed) {
                    "▶️".green()
                } else if self.audio_state.loop_recording[index]
                    .load(std::sync::atomic::Ordering::Relaxed)
                {
                    "⏺️".black()
                } else {
                    "⏹️".black()
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
