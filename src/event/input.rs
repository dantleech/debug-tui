use crossterm::event::Event;
use crossterm::event::EventStream;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use futures::FutureExt;
use futures::StreamExt;
use log::info;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::select;
use tokio::sync::mpsc::Sender;
use tokio::task;

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
    ChannelLog(String, String),
    RestartProcess,
    NotifyError(String),
}

pub type EventSender = Sender<AppEvent>;

pub fn start(event_sender: EventSender) {
    let sender = event_sender.clone();
    task::spawn(async move {
        sender.send(AppEvent::Startup).await.unwrap();
        let mut reader = EventStream::new();
        let mut tick_interval = tokio::time::interval(Duration::from_millis(1000));
        loop {
            let tick = tick_interval.tick().fuse();
            let event = reader.next().fuse();

            select! {
                _ = tick => {
                    sender.send(AppEvent::Tick).await.unwrap();
                },
                maybe_event = event => {
                    match maybe_event {
                        Some(Ok(Event::Key(key))) => {
                            info!("{:?}", key);
                            let action: Option<AppEvent> = match key.modifiers {
                                KeyModifiers::CONTROL => match key.code {
                                    KeyCode::Char('c') => Some(AppEvent::Quit),
                                    _ => None,
                                },
                                _ => None,
                            };

                            match action {
                                Some(a) => event_sender.send(a).await.unwrap(),
                                None => event_sender.send(AppEvent::Input(key)).await.unwrap(),
                            }
                        }
                        Some(Err(e)) => {
                            event_sender.send(AppEvent::NotifyError(e.to_string())).await.unwrap()
                        },
                        _ => {},
                    }
                }
            }
        }
    });
}
