use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
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
}

#[derive(Debug)]
pub struct LoopState {
    pub bar_count: u32,
    pub starting: bool,
}

impl Default for LoopState {
    fn default() -> Self {
        LoopState {
            bar_count: 4,
            starting: false,
        }
    }
}

impl Default for SetUpState {
    fn default() -> Self {
        SetUpState {
            mbpm: 120000,
            precision: 10000,
            exit: false,
            next_phase: false,
            selected: 0,
            loops: vec![LoopState {
                bar_count: 4,
                starting: true,
            }],
        }
    }
}

impl SetUpState {
    pub fn handle_events(&mut self) -> Result<()> {
        match event::read()? {
            // it's important to check that the event is a key press event as
            // crossterm also emits key release and repeat events on Windows.
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => {}
        };
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
            KeyCode::Char('l') => self.add_loop(),
            _ => {}
        }
    }

    /// Add a new loop to the list of loops
    fn add_loop(&mut self) {
        if self.loops.len() >= 4 {
            return;
        }
        let new_loop = LoopState {
            bar_count: 4,
            starting: false,
        };
        self.loops.push(new_loop);
    }

    fn decrement(&mut self) {
        if self.selected == 0 {
            self.decrement_bpm();
        } else if let Some(loop_state) = self.loops.get_mut(self.selected - 1) {
            loop_state.bar_count = 1.max(loop_state.bar_count - 1);
        }
    }

    fn increment(&mut self) {
        if self.selected == 0 {
            self.increment_bpm();
        } else if let Some(loop_state) = self.loops.get_mut(self.selected - 1) {
            loop_state.bar_count = 16.min(loop_state.bar_count + 1);
        }
    }

    /// Increment the BPM by the current precision, while keeping the maximum bpm to 3000
    fn increment_bpm(&mut self) {
        self.mbpm = 3000000.min(self.mbpm + self.precision);
    }

    /// Decrease the BPM by the current precision, while keeping the minimum bpm to 1
    fn decrement_bpm(&mut self) {
        self.mbpm = 1000.max(self.mbpm - self.precision);
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
            " Decrement ".into(),
            "<Left>".blue().bold(),
            " Increment ".into(),
            "<Right>".blue().bold(),
            if self.selected == 0 {
                " Precision ".into()
            } else {
                " Autostart ".into()
            },
            "<Tab>".blue().bold(),
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
                format!("{} bars", loop_state.bar_count).yellow(),
            ]);
            texts.push(loop_text);
        }

        Paragraph::new(Text::from(texts))
            .centered()
            .block(block)
            .render(area, buf);
    }
}
