use crate::config::Config;
use crate::dbgp::client::ContextGetResponse;
use crate::dbgp::client::ContinuationResponse;
use crate::dbgp::client::DbgpClient;
use crate::dbgp::client::StackGetResponse;
use crate::event::input::AppEvent;
use crate::event::input::ServerStatus;
use crate::notification::Notification;
use crate::view::layout::LayoutView;
use crate::view::listen::ListenView;
use crate::view::session::SessionView;
use crate::view::session::SessionViewMode;
use crate::view::session::SessionViewState;
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
use tokio::sync::Mutex;
use std::fmt::Display;
use std::io;
use std::ops::DerefMut;
use std::sync::Arc;
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
    pub stack: StackGetResponse,
    pub context: ContextGetResponse,
}

pub struct History {
    pub entries: Vec<HistoryEntry>,
    pub offset: usize,
}
impl History {
    fn default() -> History {
        Self {
            entries: vec![],
            offset: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.len() == 0
    }

    fn next(&mut self) {
        let offset = self.offset + 1;
        if offset >= self.entries.len() - 1 {
            self.offset = self.entries.len() - 1;
            return;
        }
        self.offset = offset.min(self.entries.len() - 1);
    }

    fn is_current(&self) -> bool {
        self.offset == self.entries.len() - 1
    }

    fn previous(&mut self) {
        self.offset = self.offset.saturating_sub(1);
    }

    pub(crate) fn current(&self) -> Option<&HistoryEntry> {
        self.entries.get(self.offset)
    }

    fn push(&mut self, entry: HistoryEntry) {
        self.entries.push(entry);
        self.offset = self.entries.len() - 1;
    }

    fn push_source(&mut self, filename: String, source: String) {
        self.push(HistoryEntry {
            source: SourceContext {
                source,
                filename,
                line_no: 1,
            },
            context: ContextGetResponse { properties: vec![] },
            stack: StackGetResponse { entries: vec![] },
        });
    }
}

#[derive(Clone, Debug)]
pub struct SourceContext {
    pub source: String,
    pub filename: String,
    pub line_no: u32,
}

#[derive(Debug, Clone)]
pub enum CurrentView {
    Listen,
    Session,
}

pub struct App {
    pub is_connected: bool,
    pub notification: Notification,
    pub config: Config,
    receiver: Receiver<AppEvent>,
    quit: bool,
    sender: Sender<AppEvent>,

    pub input_mode: InputMode,
    pub server_status: ServerStatus,
    pub command_input: Input,
    pub command_response: Option<String>,
    pub client: Arc<Mutex<DbgpClient>>,

    pub history: History,

    pub view_current: CurrentView,
    pub session_view: SessionViewState,
    pub input_plurality: Vec<char>,
}

impl App {
    pub fn new(config: Config, receiver: Receiver<AppEvent>, sender: Sender<AppEvent>) -> App {
        let client = DbgpClient::new(None);
        App {
            is_connected: false,
            config,
            input_plurality: vec![],
            notification: Notification::none(),
            receiver,
            sender: sender.clone(),
            quit: false,
            history: History::default(),
            client: Arc::new(Mutex::new(client)),

            input_mode: InputMode::Normal,
            server_status: ServerStatus::Initial,
            command_input: Input::default(),
            command_response: None,
            view_current: CurrentView::Listen,
            session_view: SessionViewState::new(),
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
                    sender
                        .send(AppEvent::Panic(format!(
                            "Could not listen on {}",
                            config.listen.clone()
                        )))
                        .await
                        .unwrap();
                    return;
                }
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
        event: AppEvent,
    ) -> Result<()> {
        match event {
            AppEvent::Quit => self.quit = true,
            AppEvent::ExecCommand(ref cmd) => match cmd.as_str() {
                "q" => self.sender.send(AppEvent::Quit).await.unwrap(),
                _ => {
                    let mut client = self.client.lock().await;
                    if !client.is_connected() {
                        return Ok(());
                    }
                    self.command_response = Some(client.exec_raw(cmd.to_string()).await?);
                }
            },
            AppEvent::ChangeView(view) => {
                self.view_current = view;
            }
            AppEvent::ChangeSessionViewMode(mode) => {
                self.session_view.mode = mode;
            }
            AppEvent::NextPane => {
                for _ in 0..self.take_input_plurality() {
                    self.session_view.next_pane();
                }
            }
            AppEvent::Panic(message) => {
                terminal.clear().unwrap();
                terminal
                    .draw(|frame| {
                        frame.render_widget(
                            Paragraph::new(message)
                                .style(Style::default().bg(Color::Red))
                                .block(Block::default().padding(Padding::uniform(1))),
                            Rect::new(0, 0, frame.area().width, 3),
                        )
                    })
                    .unwrap();
                self.quit = true;
            }
            AppEvent::HistoryNext => {
                for _ in 0..self.take_input_plurality() {
                    self.history.next();
                    if self.history.is_current() && self.is_connected {
                        self.sender
                            .send(AppEvent::ChangeSessionViewMode(SessionViewMode::Current))
                            .await?;
                    }
                }
            }
            AppEvent::HistoryPrevious => {
                for _ in 0..self.take_input_plurality() {
                    self.history.previous();
                }
            }
            AppEvent::ClientConnected(s) => {
                if self.is_connected {
                    panic!("Client already connected!");
                }
                let mut client = self.client.lock().await;
                let response = client.deref_mut().connect(s).await?;
                self.is_connected = true;
                self.server_status = ServerStatus::Initial;
                self.view_current = CurrentView::Session;
                let source = client.source(response.fileuri.clone()).await.unwrap();
                self.history = History::default();
                self.history.push_source(response.fileuri.clone(), source);
            }
            AppEvent::Snapshot() => {
                let mut client = self.client.lock().await;
                let stack = client.deref_mut().get_stack().await?;
                if let Some(top) = stack.top_or_none() {
                    let filename = &top.filename;
                    let line_no = top.line;
                    let source_code = client.deref_mut().source(filename.to_string()).await.unwrap();
                    let source = SourceContext {
                        source: source_code,
                        filename: filename.to_string(),
                        line_no,
                    };
                    let context = client.deref_mut().context_get().await.unwrap();
                    let entry = HistoryEntry {
                        source,
                        stack,
                        context,
                    };
                    self.history.push(entry);
                    self.session_view.reset();
                }
            }
            AppEvent::StepOut => {
                let client = Arc::clone(&self.client);
                let sender = self.sender.clone();
                tokio::spawn(async move {
                    let r = client.lock().await.deref_mut().step_out().await;
                    Self::handle_continuation_response(sender, r).await.unwrap();
                });
            }
            AppEvent::StepInto => {
                let client = Arc::clone(&self.client);
                let sender = self.sender.clone();
                let count = self.take_input_plurality();
                tokio::spawn(async move {
                    for _ in 0..count {
                        let r = client.lock().await.deref_mut().step_into().await;
                        Self::handle_continuation_response(sender.clone(), r).await.unwrap();
                    }
                });
            }
            AppEvent::StepOver => {
                let client = Arc::clone(&self.client);
                let sender = self.sender.clone();
                let count = self.take_input_plurality();
                tokio::spawn(async move {
                    for _ in 0..count {
                        let r = client.lock().await.deref_mut().step_over().await;
                        Self::handle_continuation_response(sender.clone(), r).await.unwrap();
                    }
                });
            }
            AppEvent::Run => {
                let client = Arc::clone(&self.client);
                let sender = self.sender.clone();
                let count = self.take_input_plurality();
                tokio::spawn(async move {
                    for _ in 0..count {
                        let r = client.lock().await.deref_mut().run().await;
                        Self::handle_continuation_response(sender.clone(), r).await.unwrap();
                    }
                });
            }
            AppEvent::ScrollSource(amount) => {
                self.session_view.source_scroll = self
                    .session_view
                    .source_scroll
                    .saturating_add_signed(amount * self.take_input_plurality() as i16);
            }
            AppEvent::ScrollContext(amount) => {
                self.session_view.context_scroll = self
                    .session_view
                    .context_scroll
                    .saturating_add_signed(amount * self.take_input_plurality() as i16);
            }
            AppEvent::ScrollStack(amount) => {
                self.session_view.stack_scroll = self
                    .session_view
                    .stack_scroll
                    .saturating_add_signed(amount * self.take_input_plurality() as i16);
            }
            AppEvent::ToggleFullscreen => {
                self.session_view.full_screen = !self.session_view.full_screen;
            }
            AppEvent::Input(e) => match self.input_mode {
                InputMode::Normal => match e.code {
                    KeyCode::Char(char) => match char {
                        ':' => self.input_mode = InputMode::Command,
                        _ => self.send_event_to_current_view(event).await,
                    },
                    _ => self.send_event_to_current_view(event).await,
                },
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
                let _ = self.client.lock().await.deref_mut().disonnect().await;
                self.is_connected = false;
                self.sender
                    .send(AppEvent::ChangeSessionViewMode(SessionViewMode::History))
                    .await?;
            }
            AppEvent::PushInputPlurality(char) => self.input_plurality.push(char),
            _ => self.send_event_to_current_view(event).await,
        };

        Ok(())
    }

    async fn handle_continuation_response(
        sender: Sender<AppEvent>,
        r: Result<ContinuationResponse, anyhow::Error>,
    ) -> Result<()> {
        match r {
            Ok(continuation_response) => {
                match continuation_response.status.as_str() {
                    "stopping" => {
                        sender
                            .send(AppEvent::UpdateStatus(ServerStatus::Stopping))
                            .await
                            .unwrap();
                    }
                    "break" => {
                        sender
                            .send(AppEvent::UpdateStatus(ServerStatus::Break))
                            .await
                            .unwrap();
                    }
                    _ => {
                        sender
                            .send(AppEvent::UpdateStatus(ServerStatus::Unknown(
                                continuation_response.status,
                            )))
                            .await
                            .unwrap();
                    }
                }
                // update the source code
                sender.send(AppEvent::Snapshot()).await.unwrap();
                Ok(())
            }
            Err(e) => {
                anyhow::bail!(e);
            }
        }
    }
    async fn send_event_to_current_view(&mut self, event: AppEvent) {
        let subsequent_event = match self.view_current {
            CurrentView::Listen => ListenView::handle(self, event),
            CurrentView::Session => SessionView::handle(self, event),
        };
        if let Some(event) = subsequent_event {
            self.sender.send(event).await.unwrap()
        };
    }

    pub fn take_input_plurality(&mut self) -> u8 {
        if self.input_plurality.is_empty() {
            return 1;
        }
        let input = String::from_iter(&self.input_plurality);
        self.input_plurality = Vec::new();
        match input.parse::<u8>() {
            Ok(i) => i.min(15),
            Err(e) => {
                self.notification = Notification::error(e.to_string());
                1
            }
        }
    }
}
