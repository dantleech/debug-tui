use std::{fmt::Display, io};

use crossterm::event::KeyCode;
use ratatui::{prelude::CrosstermBackend, Terminal};
use tokio::{
    net::TcpListener,
    sync::mpsc::{Receiver, Sender},
    task,
};

use crate::{
    dbgp::client::DbgpClient,
    event::input::{AppEvent, ServerStatus},
    session::Session,
    ui::render,
};

pub enum AppState {
    Listening,
    Connected,
}

#[derive(PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Command,
}
impl Display for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            AppState::Listening => "Listening",
            AppState::Connected => "Connected",
        })
    }
}

#[derive(Clone)]
struct Config {
    pub port: u16,
}

impl Config {
    pub fn new() -> Config {
        Config { port: 9003 }
    }
}

pub struct SourceContext {
    pub source: String,
    pub filename: String,
    pub line_no: u32,
}

pub struct App {
    pub state: AppState,
    pub config: Config,
    receiver: Receiver<AppEvent>,
    sender: Sender<AppEvent>,
    session: Option<Session>,
    pub input_mode: InputMode,
    pub source: Option<SourceContext>,
    pub server_status: ServerStatus,
}

impl App {
    pub fn new(receiver: Receiver<AppEvent>, sender: Sender<AppEvent>) -> App {
        App {
            config: Config::new(),
            state: AppState::Listening,
            receiver,
            sender,
            session: None,
            source: None,
            input_mode: InputMode::Normal,
            server_status: ServerStatus::Initial,
        }
    }

    pub async fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> Result<(), anyhow::Error> {
        let sender = self.sender.clone();
        let config = self.config.clone();

        // spawn connection listener co-routine
        task::spawn(async move {
            let listener = TcpListener::bind(format!("0.0.0.0:{}", config.port))
                .await
                .unwrap();

            loop {
                match listener.accept().await {
                    Ok(s) => {
                        sender.send(AppEvent::ClientConnected(s.0)).await.unwrap();
                    }
                    Err(_) => panic!("Could not connect"),
                }
            }
        });

        loop {
            let event = self.receiver.recv().await;

            if event.is_none() {
                continue;
            }

            let event = event.unwrap();

            match event {
                AppEvent::Quit => return Ok(()),
                AppEvent::Input(e) => {
                    match self.input_mode {
                        InputMode::Normal => {
                            if let KeyCode::Char(char) = e.code {
                                match char {
                                    ':' => {
                                        self.input_mode = InputMode::Command
                                    },
                                    _ => (),
                                }
                            }
                        },
                        InputMode::Command => {
                            if let KeyCode::Esc = e.code {
                                self.input_mode = InputMode::Normal
                            }
                        },
                    }
                }
                _ => (),
            };

            match self.state {
                AppState::Listening => match event {
                    AppEvent::ClientConnected(s) => {
                        let mut session = Session::new(DbgpClient::new(s), self.sender.clone());
                        let init = session.init().await?;
                        self.session = Some(session);
                        self.state = AppState::Connected;
                        self.server_status = ServerStatus::Initial;
                        self.sender
                            .send(AppEvent::RefreshSource(init.fileuri, 1))
                            .await?;
                    }
                    _ => (),
                },
                AppState::Connected => match event {
                    AppEvent::UpdateStatus(s) => {
                        self.server_status = s.clone();
                        match s {
                            ServerStatus::Break => (),
                            ServerStatus::Stopping => {
                                self.sender.send(AppEvent::Disconnect).await?;
                            }
                            _ => (),
                        }
                    }
                    AppEvent::Disconnect => {
                        self.session
                            .as_mut()
                            .expect("Session not set but it should be")
                            .disconnect()
                            .await;
                        self.session = None;
                        self.state = AppState::Listening;
                    }
                    AppEvent::UpdateSourceContext(source, filename, line_no) => {
                        self.source = Some(SourceContext { source, filename, line_no });
                    }
                    AppEvent::Input(e) => {
                        if self.input_mode == InputMode::Command {
                            ()
                        }
                        if let KeyCode::Char(char) = e.code {
                            match char {
                                'r' => self.sender.send(AppEvent::Run).await?,
                                'n' => self.sender.send(AppEvent::StepInto).await?,
                                'N' => self.sender.send(AppEvent::StepOver).await?,
                                _ => (),
                            }
                        }
                    }
                    _ => {
                        self.session
                            .as_mut()
                            .expect("Session not set but it should be")
                            .handle(event)
                            .await?
                    }
                },
            }

            terminal.autoresize()?;
            terminal.draw(|frame| {
                render(self, frame);
            })?;
        }
    }
}
