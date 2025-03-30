use std::{fmt::Display, thread, time::Duration};

use crossterm::event::{self, poll, Event, KeyCode, KeyEvent, KeyModifiers};
use tokio::{net::TcpStream, sync::mpsc::Sender};

#[derive(Debug)]
pub enum AppEvent {
    Input(KeyEvent),
    Tick,
    Quit,
    ClientConnected(TcpStream),
    UpdateStatus(ServerStatus),
    Run,
    StepInto,
    Disconnect,
    Startup,
    UpdateSourceContext(String, String, u32),
    RefreshSource(String, u32),
    StepOver,
    SessionStarted,
}

#[derive(Debug, Clone)]
pub enum ServerStatus {
    Break,
    Stopping,
    Unknown(String),
    Initial,
}

impl Display for ServerStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub type EventSender = Sender<AppEvent>;

pub fn start(event_sender: EventSender) {
    thread::spawn(move || {
        event_sender.blocking_send(AppEvent::Startup).unwrap();
        loop {
            if poll(Duration::from_millis(1000)).unwrap() {
                // handle global keys
                if let Event::Key(key) = event::read().unwrap() {
                    let action:Option<AppEvent> = match key.modifiers {
                        KeyModifiers::CONTROL => match key.code {
                            KeyCode::Char('c') => Some(AppEvent::Quit),
                            _ => None
                        }
                        _ => None
                    };
                    
                    match action {
                        Some(a) => event_sender.blocking_send(a).unwrap(),
                        None => event_sender.blocking_send(AppEvent::Input(key)).unwrap(),
                    }
                }
            }
        }
    });
}


