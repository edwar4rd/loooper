use color_eyre::Result;
use loooper::{CountInState, PrepareState, RollingState, SetUpState, audio};
use ratatui::{DefaultTerminal, Frame};

#[derive(Debug)]
pub enum State {
    SetUp(SetUpState),
    Prepare(PrepareState),
    CountIn(CountInState),
    Rolling(RollingState),
}

impl Default for State {
    fn default() -> Self {
        State::SetUp(SetUpState::default())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install().inspect_err(|_| {
        eprintln!("Failed to install color_eyre");
    })?;
    let _ = audio::host_device_setup().inspect_err(|err| {
        eprintln!("Failed to setup audio host device: {}", err);
        eprintln!("Is JACK started or pw-jack used?");
    })?;
    let terminal = ratatui::init();
    let mut state = State::default();

    let result = state.run(terminal);
    ratatui::restore();
    result
}

impl State {
    fn run(&mut self, mut terminal: DefaultTerminal) -> Result<()> {
        while !self.exiting() {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
            if self.phase_changing() {
                match self {
                    State::SetUp(state) => {
                        *self = State::Prepare(PrepareState {
                            mbpm: state.mbpm,
                            exit: false,
                            next_phase: false,
                        })
                    }
                    State::Prepare(state) => {
                        *self = State::CountIn(CountInState {
                            mbpm: state.mbpm,
                            exit: false,
                            next_phase: false,
                        })
                    }
                    State::CountIn(state) => {
                        *self = State::Rolling(RollingState {
                            mbpm: state.mbpm,
                            exit: false,
                            next_phase: false,
                        })
                    }
                    State::Rolling(state) => {
                        *self = State::SetUp(SetUpState::default())
                    }
                }
            }
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        match self {
            State::SetUp(state) => state.draw(frame),
            State::Prepare(state) => state.draw(frame),
            State::CountIn(state) => state.draw(frame),
            State::Rolling(state) => state.draw(frame),
        }
    }

    fn handle_events(&mut self) -> Result<()> {
        match self {
            State::SetUp(state) => state.handle_events(),
            State::Prepare(state) => state.handle_events(),
            State::CountIn(state) => state.handle_events(),
            State::Rolling(state) => state.handle_events(),
        }
    }

    fn phase_changing(&self) -> bool {
        match self {
            State::SetUp(state) => state.phase_changing(),
            State::Prepare(state) => state.phase_changing(),
            State::CountIn(state) => state.phase_changing(),
            State::Rolling(state) => state.phase_changing(),
        }
    }

    fn exiting(&self) -> bool {
        match self {
            State::SetUp(state) => state.exiting(),
            State::Prepare(state) => state.exiting(),
            State::CountIn(state) => state.exiting(),
            State::Rolling(state) => state.exiting(),
        }
    }
}
