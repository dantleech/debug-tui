use crossterm::event::poll;
use crossterm::event::Event;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use crossterm::event::{
    self,
};
use std::thread;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::mpsc::Sender;

use crate::app::CurrentView;
use crate::dbgp::client::ContinuationStatus;
use crate::view::session::SessionViewMode;

#[derive(Debug)]
pub enum AppEvent {
    ChangeSessionViewMode(SessionViewMode),
    ChangeView(CurrentView),
    ClientConnected(TcpStream),
    Disconnect,
    HistoryNext,
    HistoryPrevious,
    Input(KeyEvent),
    Panic(String),
    Quit,
    Run,
    SessionStarted,
    Snapshot(),
    Startup,
    StepInto,
    StepOut,
    StepOver,
    Tick,
    UpdateSourceContext(String, String, u32),
    UpdateStatus(ContinuationStatus),
    NextPane,
    ScrollDown(i16),
    ScrollUp(i16),
    ScrollSource(i16),
    ScrollContext(i16),
    ToggleFullscreen,
    ScrollStack(i16),
    PushInputPlurality(char),
    ContextDepth(i8),
    NextTheme,
}

pub type EventSender = Sender<AppEvent>;

pub fn start(event_sender: EventSender) {
    thread::spawn(move || {
        event_sender.blocking_send(AppEvent::Startup).unwrap();
        loop {
            if poll(Duration::from_millis(1000)).unwrap() {
                // handle global keys
                if let Event::Key(key) = event::read().unwrap() {
                    let action: Option<AppEvent> = match key.modifiers {
                        KeyModifiers::CONTROL => match key.code {
                            KeyCode::Char('c') => Some(AppEvent::Quit),
                            _ => None,
                        },
                        _ => None,
                    };

                    match action {
                        Some(a) => event_sender.blocking_send(a).unwrap(),
                        None => event_sender.blocking_send(AppEvent::Input(key)).unwrap(),
                    }
                }
            }
            event_sender.blocking_send(AppEvent::Tick).unwrap();
        }
    });
}
