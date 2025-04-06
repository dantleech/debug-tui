use crate::dbgp::client::ContinuationResponse;
use crate::dbgp::client::DbgpClient;
use crate::event::input::AppEvent;
use crate::event::input::ServerStatus;
use crate::notification::Notification;
use crate::view::history::HistoryView;
use crate::view::layout::LayoutView;
use crate::view::listen::ListenView;
use crate::view::session::SessionView;
use crate::view::View;
use anyhow::Result;
use crossterm::event::Event;
use crossterm::event::KeyCode;
use ratatui::prelude::CrosstermBackend;
use ratatui::Terminal;
use std::fmt::Display;
use std::io;
use tokio::net::TcpListener;
use tokio::sync::mpsc::Receiver;
use tokio::sync::mpsc::Sender;
use tokio::task;
use tui_input::backend::crossterm::EventHandler;
use tui_input::Input;

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

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

impl Config {
    pub fn new() -> Config {
        Config { port: 9003 }
    }
}

#[derive(Clone, Debug)]
pub struct SourceContext {
    pub source: String,
    pub filename: String,
    pub line_no: u32,
}

#[derive(Debug)]
pub enum CurrentView {
    Listen,
    Session,
    History,
}

pub struct Views {}
pub struct AppState {}
pub struct App {
    pub notification: Notification,
    pub config: Config,
    receiver: Receiver<AppEvent>,
    quit: bool,
    sender: Sender<AppEvent>,
    pub input_mode: InputMode,
    pub source: Option<SourceContext>,
    pub server_status: ServerStatus,
    pub command_input: Input,
    pub command_response: Option<String>,
    pub client: DbgpClient,

    pub history: Vec<SourceContext>,
    pub history_offset: usize,

    pub view_current: CurrentView,
    pub view_listen: ListenView,
    pub view_session: SessionView,
}

impl App {
    pub fn new(receiver: Receiver<AppEvent>, sender: Sender<AppEvent>) -> App {
        let client = DbgpClient::new(None);
        App {
            config: Config::new(),
            notification: Notification::none(),
            receiver,
            sender: sender.clone(),
            quit: false,
            history: vec![],
            history_offset: 0,
            client,
            source: None,
            input_mode: InputMode::Normal,
            server_status: ServerStatus::Initial,
            command_input: Input::default(),
            command_response: None,
            view_current: CurrentView::Listen,
            view_listen: ListenView {},
            view_session: SessionView::new(),
        }
    }

    pub async fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> Result<()> {
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
                return Ok(());
            }

            terminal.autoresize()?;
            terminal.draw(|frame| {
                LayoutView::draw(self, frame, frame.area());
            })?;
        }
    }

    async fn handle_event(&mut self, event: AppEvent) -> Result<()> {
        match event {
            AppEvent::Quit => self.quit = true,
            AppEvent::ExecCommand(ref cmd) => match cmd.as_str() {
                "q" => self.sender.send(AppEvent::Quit).await.unwrap(),
                _ => {
                    if !self.client.is_connected() {
                        return Ok(());
                    }
                    self.command_response = Some(self.client.exec_raw(cmd.to_string()).await?);
                }
            },
            AppEvent::ChangeView(view) => {
                self.view_current = view;
            },
            AppEvent::HistoryNext => {
                let offset = self.history_offset + 1;
                self.history_offset = offset.min(self.history.len() - 1);
            },
            AppEvent::HistoryPrevious => {
                self.history_offset = self.history_offset.saturating_sub(1);
            },
            AppEvent::ClientConnected(s) => {
                let response = self.client.connect(s).await?;
                self.server_status = ServerStatus::Initial;
                self.view_current = CurrentView::Session;
                self.history = vec![];
                self.sender
                    .send(AppEvent::PushSource(response.fileuri, 1))
                    .await
                    .unwrap()
            }
            AppEvent::PushSource(ref filename, line_no) => {
                let source = self.client.source(filename.clone()).await.unwrap();
                let source_context = SourceContext {
                    source,
                    filename: filename.clone(),
                    line_no,
                };
                self.history.push(source_context.clone());
                self.history_offset = self.history.len() - 1;
                self.source = Some(source_context);
            }
            AppEvent::StepInto => {
                let response = self.client.step_into().await?;
                self.handle_continuation_response(response).await?;
            }
            AppEvent::StepOver => {
                let response = self.client.step_over().await?;
                self.handle_continuation_response(response).await?;
            }
            AppEvent::Run => {
                let response = self.client.run().await?;
                self.handle_continuation_response(response).await?;
            }
            AppEvent::Input(e) => match self.input_mode {
                InputMode::Normal => {
                    if let KeyCode::Char(char) = e.code {
                        match char {
                            ':' => self.input_mode = InputMode::Command,
                            _ => self.send_event_to_current_view(event).await,
                        }
                    }
                }
                InputMode::Command => match e.code {
                    // escape back to normal mode
                    KeyCode::Esc => {
                        self.input_mode = InputMode::Normal;
                        self.command_response = None;
                    }
                    // execute command
                    KeyCode::Enter => {
                        self.input_mode = InputMode::Normal;
                        self.sender
                            .send(AppEvent::ExecCommand(
                                self.command_input.value().to_string(),
                            ))
                            .await
                            .unwrap();
                    }
                    // delegate keys to command input
                    _ => {
                        self.command_input.handle_event(&Event::Key(e));
                    }
                },
            },
            // needed?
            AppEvent::UpdateStatus(server_status) => {
                self.server_status = server_status.clone();
                match server_status {
                    ServerStatus::Break => (),
                    ServerStatus::Stopping => {
                        self.sender.send(AppEvent::Disconnect).await.unwrap();
                    }
                    _ => (),
                }
            }
            AppEvent::Disconnect => {
                self.client.disonnect().await;
            }
            AppEvent::UpdateSourceContext(source, filename, line_no) => {
                self.source = Some(SourceContext {
                    source,
                    filename,
                    line_no,
                });
            }
            _ => self.send_event_to_current_view(event).await,
        };

        Ok(())
    }

    async fn handle_continuation_response(&mut self, r: ContinuationResponse) -> Result<()> {
        match r.status.as_str() {
            "stopping" => {
                self.sender
                    .send(AppEvent::UpdateStatus(ServerStatus::Stopping))
                    .await
                    .unwrap();
            }
            "break" => {
                self.sender
                    .send(AppEvent::UpdateStatus(ServerStatus::Break))
                    .await
                    .unwrap();
            }
            _ => {
                self.sender
                    .send(AppEvent::UpdateStatus(ServerStatus::Unknown(r.status)))
                    .await
                    .unwrap();
            }
        }
        // update the source code
        let stack = self.client.get_stack().await?;
        if let Some(stack) = stack {
            self.sender
                .send(AppEvent::PushSource(stack.filename, stack.line))
                .await
                .unwrap();
        };
        Ok(())
    }
    async fn send_event_to_current_view(&mut self, event: AppEvent) {
        let subsequent_event = match self.view_current {
            CurrentView::Listen => ListenView::handle(self, event),
            CurrentView::Session => SessionView::handle(self, event),
            CurrentView::History => HistoryView::handle(self, event),
        };
        if let Some(event) = subsequent_event { self.sender.send(event).await.unwrap() };
    }
}
