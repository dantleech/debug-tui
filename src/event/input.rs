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

use crate::app::SelectedView;
use crate::dbgp::client::ContinuationStatus;
use crate::view::session::SessionViewMode;
use crate::view::Scroll;

#[derive(Debug)]
pub enum AppEvent {
    ChangeSessionViewMode(SessionViewMode),
    ChangeView(SelectedView),
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
    PreviousPane,
    Scroll(Scroll),
    ScrollSource(Scroll),
    ScrollContext(Scroll),
    ScrollStack(Scroll),
    ScrollEval(Scroll),
    ToggleFullscreen,
    PushInputPlurality(char),
    ContextDepth(i8),
    NextTheme,
    ContextFilterOpen,
    ContextSearchClose,
    Listen,
    EvalCancel,
    EvalExecute,
    EvalRefresh,
    EvalStart,
    Listening,
    NextChannel,
    FocusChannel(String),
    ChannelLog(String,String),
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
