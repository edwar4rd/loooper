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
pub struct SetUpState {
    /// The beats per minute (BPM).
    pub mbpm: u32,
    /// The precision of the BPM adjustment.
    pub precision: u32,
    /// Whether to exit the application.
    pub exit: bool,
    /// Whether to enter the prepare phase.
    pub next_phase: bool,
    /// The selected loop index.
    pub selected: usize,
    /// The list of loops.
    pub loops: Vec<LoopState>,
    /// The event stream for receiving terminal events.
    pub event_stream: EventStream,
    /// The audio state.
    pub audio_state: AudioState,
    /// The serial number of the last error.
    error_count: usize,
    /// The last error message.
    last_error: String,
}

#[derive(Debug)]
pub struct LoopState {
    /// The length of the loop in beats.
    pub beat_count: u32,
    /// Whether the loop should start immediately after count-in.
    pub starting: bool,
    /// Whether the loop should be layered on top of prievious recording.
    pub layering: bool,
}

impl Default for LoopState {
    fn default() -> Self {
        LoopState {
            beat_count: 4,
            starting: false,
            layering: false,
        }
    }
}

impl SetUpState {
    pub fn default_with_audio_state(audio_state: crate::audio::AudioState) -> Self {
        SetUpState {
            mbpm: 120000,
            precision: 10000,
            exit: false,
            next_phase: false,
            selected: 0,
            loops: vec![LoopState {
                beat_count: 4,
                starting: true,
                layering: false,
            }],
            event_stream: EventStream::new(),
            audio_state,
            error_count: 0,
            last_error: String::new(),
        }
    }

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
            maybe_error = self.audio_state.messages.recv() => {
                if let Some(error) = maybe_error {
                    self.last_error = error;
                    self.error_count += 1;
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

    pub fn from_rolling_state(rolling_state: crate::RollingState) -> Self {
        rolling_state
            .audio_state
            .enabled
            .store(false, std::sync::atomic::Ordering::Relaxed);
        SetUpState {
            mbpm: rolling_state.mbpm,
            precision: 10000,
            exit: false,
            next_phase: false,
            selected: 0,
            loops: vec![LoopState {
                beat_count: 4,
                starting: true,
                layering: false,
            }],
            event_stream: rolling_state.event_stream,
            audio_state: rolling_state.audio_state,
            error_count: 0,
            last_error: String::new(),
        }
    }
}

impl SetUpState {
    fn exit(&mut self) {
        self.exit = true;
    }

    fn transititon(&mut self) {
        self.next_phase = true;
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Left => self.decrement(),
            KeyCode::Right => self.increment(),
            KeyCode::Up => self.select_priv(),
            KeyCode::Down => self.select_next(),
            KeyCode::Tab => {
                if self.selected == 0 {
                    self.change_precision()
                } else {
                    self.toggle_autostart()
                }
            }
            KeyCode::Char(' ') => self.transititon(),
            KeyCode::Char('p') => panic!("Manual panic!"),
            KeyCode::Char('a') => self.add_loop(),
            KeyCode::Char('l') => self.toggle_layering(),
            _ => {}
        }
    }

    /// Add a new loop to the list of loops
    fn add_loop(&mut self) {
        if self.loops.len() >= 8 {
            return;
        }
        let new_loop = LoopState {
            beat_count: 4,
            starting: false,
            layering: false,
        };
        self.loops.push(new_loop);
    }

    fn decrement(&mut self) {
        if self.selected == 0 {
            self.decrement_bpm();
        } else if let Some(loop_state) = self.loops.get_mut(self.selected - 1) {
            loop_state.beat_count = 1.max(loop_state.beat_count - 1);
        }
    }

    fn increment(&mut self) {
        if self.selected == 0 {
            self.increment_bpm();
        } else if let Some(loop_state) = self.loops.get_mut(self.selected - 1) {
            loop_state.beat_count = 16.min(loop_state.beat_count + 1);
        }
    }

    /// Increment the BPM by the current precision, while keeping the maximum bpm to 3000
    fn increment_bpm(&mut self) {
        self.mbpm = 3000000.min(self.mbpm + self.precision);
    }

    /// Decrease the BPM by the current precision, while keeping the minimum bpm to 1
    fn decrement_bpm(&mut self) {
        self.mbpm = 1000.max(self.mbpm.saturating_sub(self.precision));
    }

    fn change_precision(&mut self) {
        if self.precision == 10000 {
            self.precision = 1000;
        } else if self.precision == 1000 {
            self.precision = 100;
        } else if self.precision == 100 {
            self.precision = 10;
        } else {
            self.precision = 10000;
        }
    }

    fn select_next(&mut self) {
        if self.selected == 0 {
            self.selected = 1;
        } else if self.selected >= self.loops.len() {
            self.selected = 0;
        } else {
            self.selected += 1;
        }
    }

    fn select_priv(&mut self) {
        if self.selected == 0 {
            self.selected = self.loops.len();
        } else {
            self.selected -= 1;
        }
    }

    fn toggle_autostart(&mut self) {
        if self.selected == 0 {
            return;
        }
        if let Some(loop_state) = self.loops.get_mut(self.selected - 1) {
            loop_state.starting = !loop_state.starting;
        }
    }

    fn toggle_layering(&mut self) {
        if self.selected == 0 {
            return;
        }
        if let Some(loop_state) = self.loops.get_mut(self.selected - 1) {
            loop_state.layering = !loop_state.layering;
        }
    }
}

impl Widget for &SetUpState {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Line::from(vec![
            " L".bold(),
            "O".set_style(Color::Rgb(26, 153, 136)).bold().italic(),
            "O".set_style(Color::Rgb(17, 85, 204)).bold().italic(),
            "O".set_style(Color::Rgb(180, 95, 6)).bold().italic(),
            "PER ".bold(),
            "(setup) ".italic(),
        ]);
        let instructions = Line::from(vec![
            if self.selected == 0 {
                " Precision ".into()
            } else {
                " Autostart ".into()
            },
            "<Tab>".blue().bold(),
            if self.selected == 0 {
                "".into()
            } else {
                " Toggle Layering ".into()
            },
            if self.selected == 0 {
                "".into()
            } else {
                "<L>".blue().bold()
            },
            " Add Loop ".into(),
            "<A>".blue().bold(),
            " Finish Setup ".into(),
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
        let counter_line = Line::from(vec![
            if self.selected == 0 {
                ">> ".green()
            } else {
                "".into()
            },
            "BPM: ".into(),
            bpm.to_string().yellow(),
            format!(" (+/-{})", self.precision as f32 / 1000.).italic(),
        ]);
        texts.push(counter_line);
        for (i, loop_state) in self.loops.iter().enumerate() {
            let loop_text = Line::from(vec![
                if self.selected == i + 1 {
                    ">> ".green()
                } else {
                    "".into()
                },
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
        texts.push(Line::from(vec![
            format!("Message #{}: ", self.error_count).into(),
            self.last_error.as_str().into(),
        ]));

        Paragraph::new(Text::from(texts))
            .centered()
            .block(block)
            .render(area, buf);
    }
}
