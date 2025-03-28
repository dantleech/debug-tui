use crossterm::event::KeyCode;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::event::input::AppEvent;

pub enum AppState {
    Listening,
    Connected,
}

struct Config {
    pub port: u16,
}

impl Config {
    pub fn new() -> Config {
        Config { port: 9003 }
    }
}

pub struct App {
    connection: AppState,
    config: Config,
    receiver: Receiver<AppEvent>,
    sender: Sender<AppEvent>,
    quit: bool,
}

impl App {
    pub fn new(receiver: Receiver<AppEvent>, sender: Sender<AppEvent>) -> App {
        App {
            config: Config::new(),
            connection: AppState::Listening,
            receiver,
            sender,
            quit: false,
        }
    }

    pub async fn run(&mut self) {
        loop {
            let event = self.receiver.recv().await;
            if self.quit == true {
                return;
            }

            if event.is_none() {
                continue;
            }

            let event = event.unwrap();

            match event {
                AppEvent::Input(inkey) => {
                    if inkey.code == KeyCode::Char('c') && inkey.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                        self.quit = true;
                    }
                },
                AppEvent::Tick => continue,
            };
        }
    }
}
