pub mod app;
pub mod config;
pub mod dbgp;
pub mod event;
pub mod notification;
pub mod view;

use app::App;
use better_panic::Settings;
use config::load_config;
use crossterm::terminal::disable_raw_mode;
use event::input;
use ratatui::crossterm::terminal::enable_raw_mode;
use ratatui::prelude::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use std::panic;
use std::process;
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
    enable_raw_mode()?;
    set_panic_hook();
    terminal.clear()?;
    let (event_sender, event_receiver) = mpsc::channel(32);

    // start input thread
    input::start(event_sender.clone());
    let config = load_config();

    let mut app = App::new(config, event_receiver, event_sender);
    app.run(&mut terminal).await?;

    disable_raw_mode()?;

    Ok(())
}

fn set_panic_hook() {
    std::panic::set_hook(Box::new(|panic_info| {
        crossterm::execute!(std::io::stderr(), crossterm::terminal::LeaveAlternateScreen).unwrap();
        crossterm::terminal::disable_raw_mode().unwrap();
        Settings::auto()
            .most_recent_first(false)
            .lineno_suffix(true)
            .create_panic_handler()(panic_info);
    }));
}
