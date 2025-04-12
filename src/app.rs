use crate::config::Config;
use crate::dbgp::client::ContextGetResponse;
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
use ratatui::layout::Rect;
use ratatui::prelude::CrosstermBackend;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::widgets::Block;
use ratatui::widgets::Padding;
use ratatui::widgets::Paragraph;
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

#[derive(Clone, Debug)]
pub struct HistoryEntry {
    pub source: SourceContext,
    pub context: ContextGetResponse,
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
    pub server_status: ServerStatus,
    pub command_input: Input,
    pub command_response: Option<String>,
    pub client: DbgpClient,

    pub history: Vec<HistoryEntry>,
    pub history_offset: usize,

    pub view_current: CurrentView,
    pub view_listen: ListenView,
    pub view_session: SessionView,
}

impl App {
    pub fn new(
        config: Config,
        receiver: Receiver<AppEvent>,
        sender: Sender<AppEvent>
    ) -> App {
        let client = DbgpClient::new(None);
        App {
            config,
            notification: Notification::none(),
            receiver,
            sender: sender.clone(),
            quit: false,
            history: vec![],
            history_offset: 0,
            client,

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
            let listener = match TcpListener::bind(config.listen.clone()).await {
                Ok(l) => l,
                Err(_) => {
                    sender.send(AppEvent::Panic(
                        format!("Could not listen on {}", config.listen.clone())
                    )).await.unwrap();
                    return;
                },
            };

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

            self.handle_event(terminal, event).await?;

            if self.quit {
                return Ok(());
            }

            terminal.autoresize()?;
            terminal.draw(|frame| {
                LayoutView::draw(self, frame, frame.area());
            })?;
        }
    }

    async fn handle_event(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        event: AppEvent
    ) -> Result<()> {
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
            AppEvent::Panic(message) => {
                terminal.clear().unwrap();
                terminal.draw(|frame|{
                    frame.render_widget(
                        Paragraph::new(
                            message
                        ).style(Style::default().bg(Color::Red)).block(Block::default().padding(Padding::uniform(1))),
                        Rect::new(0, 0, frame.area().width, 3)
                    )
                }).unwrap();
                self.quit = true;
            }
            AppEvent::HistoryNext => {
                let offset = self.history_offset + 1;
                if offset >= self.history.len() - 1 {
                    self.sender.send(AppEvent::ChangeView(CurrentView::Session)).await?;
                    self.history_offset = self.history.len() - 1;
                    return Ok(());
                }
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
            }
            AppEvent::Snapshot() => {
                let stack = self.client.get_stack().await?;
                if let Some(stack) = stack {
                    let filename = stack.filename;
                    let line_no = stack.line;
                    let source = self.client.source(filename.clone()).await.unwrap();
                    let source_context = SourceContext {
                        source,
                        filename: filename.clone(),
                        line_no,
                    };
                    let context = self.client.context_get().await.unwrap();
                    let entry = HistoryEntry{
                        source: source_context,
                        context,
                    };
                    self.history.push(entry);
                    self.history_offset = self.history.len() - 1;
                }
            }
            AppEvent::StepOut => {
                let response = self.client.step_out().await;
                self.handle_continuation_response(response).await?;
            }
            AppEvent::StepInto => {
                let response = self.client.step_into().await;
                self.handle_continuation_response(response).await?;
            }
            AppEvent::StepOver => {
                let response = self.client.step_over().await;
                self.handle_continuation_response(response).await?;
            }
            AppEvent::Run => {
                let response = self.client.run().await;
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
                let _ = self.client.disonnect().await;
                self.sender.send(AppEvent::ChangeView(CurrentView::Listen)).await?;
            }
            _ => self.send_event_to_current_view(event).await,
        };

        Ok(())
    }

    async fn handle_continuation_response(&mut self, r: Result<ContinuationResponse, anyhow::Error>) -> Result<()> {
        match r {
            Ok(continuation_response) => {
                match continuation_response.status.as_str() {
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
                            .send(AppEvent::UpdateStatus(ServerStatus::Unknown(continuation_response.status)))
                            .await
                            .unwrap();
                    }
                }
                // update the source code
                self.sender
                    .send(AppEvent::Snapshot())
                    .await
                    .unwrap();
                Ok(())
            }
            Err(e) => {
                self.notification = Notification::error(e.to_string());
                Ok(())
            }
        }
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
