pub mod notification;

use std::{fmt::Display, io};

use crossterm::event::{Event, KeyCode};
use notification::Notification;
use ratatui::{prelude::CrosstermBackend, Terminal};
use tokio::{
    net::TcpListener,
    sync::mpsc::{Receiver, Sender},
    task,
};
use tui_input::{backend::crossterm::EventHandler, Input};

use crate::{
    dbgp::client::{DbgpClient, Response},
    event::input::{AppEvent, ServerStatus},
    session::Session,
    ui::render,
};

pub enum AppState {
    Listening,
    Connected,
}

impl Display for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            AppState::Listening => "Listening",
            AppState::Connected => "Connected",
        })
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum InputMode {
    Normal,
    Command,
}

impl Display for InputMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Clone)]
pub struct Config {
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
    pub notification: Notification,
    receiver: Receiver<AppEvent>,
    quit: bool,
    sender: Sender<AppEvent>,
    session: Option<Session>,
    pub input_mode: InputMode,
    pub source: Option<SourceContext>,
    pub server_status: ServerStatus,
    pub command_input: Input,
    pub command_response: Option<String>,
}

impl App {
    pub fn new(receiver: Receiver<AppEvent>, sender: Sender<AppEvent>) -> App {
        App {
            config: Config::new(),
            state: AppState::Listening,
            notification: Notification::none(),
            receiver,
            sender,
            quit: false,
            session: None,
            source: None,
            input_mode: InputMode::Normal,
            server_status: ServerStatus::Initial,
            command_input: Input::default(),
            command_response: None,
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

            self.handle_event(event).await?;

            if self.quit {
                return Ok(())
            }

            terminal.autoresize()?;
            terminal.draw(|frame| {
                render(self, frame);
            })?;
        }
    }

    async fn handle_event(&mut self, event: AppEvent) -> Result<(), anyhow::Error> {
        match event {
            AppEvent::Quit => self.quit = true,
            AppEvent::ExecCommand(ref cmd) => {
                match cmd.as_str() {
                    "q" => {
                        self.sender.send(AppEvent::Quit).await?;
                    }
                    // let the session handle it later
                    _ => (),
                }
            }
            AppEvent::ExecCommandResponse(ref response) => {
                self.command_response = Some(response.to_string())
            }
            AppEvent::Input(e) => match self.input_mode {
                InputMode::Normal => {
                    if let KeyCode::Char(char) = e.code {
                        match char {
                            ':' => self.input_mode = InputMode::Command,
                            _ => (),
                        }
                    }
                }
                InputMode::Command => match e.code {
                    // escape back to normal mode
                    KeyCode::Esc => {
                        self.input_mode = InputMode::Normal;
                        self.command_response = None;
                        return Ok(())
                    }
                    // execute command
                    KeyCode::Enter => {
                        self.input_mode = InputMode::Normal;
                        self.sender
                            .send(AppEvent::ExecCommand(
                                self.command_input.value().to_string(),
                            ))
                            .await?;
                        return Ok(())
                    }
                    // delegate keys to command input
                    _ => {
                        self.command_input.handle_event(&Event::Key(e));
                        return Ok(())
                    }
                },
            },
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
                    self.source = Some(SourceContext {
                        source,
                        filename,
                        line_no,
                    });
                }
                AppEvent::Input(e) => {
                    if self.input_mode != InputMode::Command {
                        if let KeyCode::Char(char) = e.code {
                            match char {
                                'r' => self.sender.send(AppEvent::Run).await?,
                                'n' => self.sender.send(AppEvent::StepInto).await?,
                                'o' => self.sender.send(AppEvent::StepOver).await?,
                                _ => (),
                            }
                        }
                    }
                }
                _ => {
                    match self.session
                        .as_mut()
                        .expect("Session not set but it should be")
                        .handle(event)
                        .await {
                            Ok(_) => (),
                            Err(e) => {
                                self.notification = Notification::error(e.to_string());
                            }
                        };
                }
            },
        };
        Ok(())
    }
}


