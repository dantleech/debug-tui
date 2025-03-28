use std::{time::Duration, thread};

use crossterm::event::{Event, KeyEvent, self, poll};
use tokio::sync::mpsc::Sender;

#[derive(Debug)]
pub enum AppEvent {
    Input(KeyEvent),
    Tick,
}

pub type EventSender = Sender<AppEvent>;

pub fn start(event_sender: EventSender) {
    thread::spawn(move || {
        loop {
            if poll(Duration::from_millis(1000)).unwrap() {
                if let Event::Key(key) = event::read().unwrap() {
                    event_sender.blocking_send(AppEvent::Input(key)).unwrap();
                }
            }

            // ignore errors from tick - it causes panics on shutdown
            let _ = event_sender.blocking_send(AppEvent::Tick);
        }
    });
}


