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
}

impl Default for SetUpState {
    fn default() -> Self {
        SetUpState {
            mbpm: 120000,
            precision: 10000,
            exit: false,
            next_phase: false,
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
            KeyCode::Left => self.decrement_bpm(),
            KeyCode::Right => self.increment_bpm(),
            KeyCode::Tab => self.change_precision(),
            KeyCode::Char(' ') => self.transititon(),
            _ => {}
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
}

impl Widget for &SetUpState {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Line::from(vec![
            " L".bold(),
            "O".set_style(Color::Rgb(26, 153, 136)).bold().italic(),
            "O".set_style(Color::Rgb(17, 85, 204)).bold().italic(),
            "O".set_style(Color::Rgb(180, 95, 6)).bold().italic(),
            "PER ".bold(),
            "(setup) ".italic()
        ]);
        let instructions = Line::from(vec![
            " Decrement ".into(),
            "<Left>".blue().bold(),
            " Increment ".into(),
            "<Right>".blue().bold(),
            " Change precision".into(),
            format!(" (Current: {}) ", self.precision as f32 / 1000.).into(),
            "<Tab>".blue().bold(),
            " Quit ".into(),
            "<Q> ".blue().bold(),
        ]);
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
