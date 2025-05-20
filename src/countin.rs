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
pub struct CountInState {
    /// The beats per minute (BPM).
    pub mbpm: u32,
    /// Whether to exit the application.
    pub exit: bool,
    /// Whether to enter the prepare phase.
    pub next_phase: bool,
}

impl Default for CountInState {
    fn default() -> Self {
        CountInState {
            mbpm: 120000,
            exit: false,
            next_phase: false,
        }
    }
}

impl CountInState {
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
        let instructions = Line::from(vec![
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
