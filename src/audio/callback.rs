pub fn create_callback() -> impl jack::ProcessHandler {
    let callback_closure =
        move |client: &jack::Client, ps: &jack::ProcessScope| jack::Control::Continue;

    jack::contrib::ClosureProcessHandler::new(callback_closure)
}
