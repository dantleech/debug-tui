use std::{time::Duration, thread};

use crossterm::event::{self, poll, Event, KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc::Sender;

#[derive(Debug)]
pub enum AppEvent {
    Input(KeyEvent),
    Tick,
    Quit,
}

pub type EventSender = Sender<AppEvent>;

pub fn start(event_sender: EventSender) {
    thread::spawn(move || {
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

            // ignore errors from tick - it causes panics on shutdown
            let _ = event_sender.blocking_send(AppEvent::Tick);
        }
    });
}


