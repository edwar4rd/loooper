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

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    color_eyre::install().inspect_err(|_| {
        eprintln!("Failed to install color_eyre");
    })?;
    let (client, audio_state) = audio::audio_setup().inspect_err(|err| {
        eprintln!("Failed to setup audio: {}", err);
        eprintln!("Is JACK started or pw-jack used?");
    })?;

    let current_millibeat = audio_state.current_millibeat.clone();
    let pad_tx = audio_state.pad_tx.clone();
    let (blink_shutdown_tx, blink_shutdown_rx) = tokio::sync::oneshot::channel();
    let (button_shutdown_tx, button_shutdown_rx) = tokio::sync::oneshot::channel();
    let (button_tx, button_rx) = tokio::sync::mpsc::unbounded_channel();

    let blink_handle =
        std::thread::spawn(move || loooper::blink::blink(current_millibeat, blink_shutdown_rx));
    let button_handle =
        std::thread::spawn(move || loooper::button::button(pad_tx, button_tx, button_shutdown_rx));

    let terminal = ratatui::init();
    let state = State::default_with_audio_state(audio_state, button_rx);

    let result = state.run(terminal).await;
    let _ = blink_shutdown_tx.send(());
    let _ = button_shutdown_tx.send(());
    drop(client);
    ratatui::restore();
    let blink_res = blink_handle.join();
    if let Ok(Err(err)) = blink_res {
        eprintln!("Blinking thread error:\t{:?}\n", err);
    }

    let button_res = button_handle.join();
    if let Ok(Err(err)) = button_res {
        eprintln!("Button input thread error:\t{:?}", err);
    }

    result
}

impl State {
    fn default_with_audio_state(
        audio_state: audio::AudioState,
        button_rx: tokio::sync::mpsc::UnboundedReceiver<usize>,
    ) -> Self {
        State::SetUp(SetUpState::default_with_audio_state(audio_state, button_rx))
    }

    async fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        while !self.exiting() {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events().await?;
            if self.phase_changing() {
                match self {
                    State::SetUp(state) => {
                        self = State::Prepare(PrepareState::from_setup_state(state))
                    }
                    State::Prepare(state) => {
                        self = State::CountIn(CountInState::from_prepare_state(state))
                    }
                    State::CountIn(state) => {
                        self = State::Rolling(RollingState::from_countin_state(state))
                    }
                    State::Rolling(state) => {
                        self = State::SetUp(SetUpState::from_rolling_state(state))
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

    async fn handle_events(&mut self) -> Result<()> {
        match self {
            State::SetUp(state) => state.handle_events().await,
            State::Prepare(state) => state.handle_events().await,
            State::CountIn(state) => state.handle_events().await,
            State::Rolling(state) => state.handle_events().await,
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
