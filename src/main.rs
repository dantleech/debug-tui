pub mod app;
pub mod event;
pub mod dbgp;
pub mod session;

use std::{io, panic, process};

use app::App;
use crossterm::terminal::disable_raw_mode;
use event::input;
use ratatui::{crossterm::terminal::enable_raw_mode, prelude::CrosstermBackend, restore, Terminal};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let orig_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        orig_hook(panic_info);
        process::exit(1);
    }));

    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal: Terminal<CrosstermBackend<io::Stdout>> = Terminal::new(backend)?;
    // enable_raw_mode()?;
    // set_panic_hook();
    terminal.clear()?;
    let (event_sender, event_receiver) = mpsc::channel(32);

    // start input thread
    input::start(event_sender.clone());

    let mut app = App::new(event_receiver, event_sender);
    app.run().await?;

    // disable_raw_mode()?;

    Ok(())
}

fn set_panic_hook() {
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = restore(); // ignore any errors as we are already failing
        hook(panic_info);
    }));
}
