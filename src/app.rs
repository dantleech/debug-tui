use crate::analyzer::Analyser;
use crate::analyzer::Analysis;
use crate::config::Config;
use crate::dbgp::client::ContextGetResponse;
use crate::dbgp::client::ContinuationResponse;
use crate::dbgp::client::ContinuationStatus;
use crate::dbgp::client::DbgpClient;
use crate::dbgp::client::Property;
use crate::event::input::AppEvent;
use crate::notification::Notification;
use crate::theme::Scheme;
use crate::theme::Theme;
use crate::view::help::HelpView;
use crate::view::layout::LayoutView;
use crate::view::listen::ListenView;
use crate::view::session::SessionView;
use crate::view::session::SessionViewMode;
use crate::view::session::SessionViewState;
use crate::view::View;
use anyhow::Result;
use crossterm::event::KeyCode;
use log::info;
use log::warn;
use ratatui::layout::Rect;
use ratatui::prelude::CrosstermBackend;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::widgets::Block;
use ratatui::widgets::Padding;
use ratatui::widgets::Paragraph;
use ratatui::Terminal;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::io;
use std::ops::DerefMut;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::mpsc::Receiver;
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;
use tokio::sync::Notify;
use tokio::task;
use tui_input::Input;

type AnalyzedFiles = HashMap<String, Analysis>;

#[derive(Clone, Debug)]
pub struct StackFrame {
    pub level: u16,
    pub source: SourceContext,
    pub context: Option<ContextGetResponse>,
}
impl StackFrame {
    pub(crate) fn get_property(&self, name: &str) -> Option<&Property> {
        match &self.context {
            Some(c) => c.properties.iter().find(|&property| property.name == name),
            None => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct HistoryEntry {
    pub stacks: Vec<StackFrame>,
}

impl HistoryEntry {
    fn push(&mut self, frame: StackFrame) {
        self.stacks.push(frame);
    }
    fn new() -> Self {
        let stacks = Vec::new();
        HistoryEntry { stacks }
    }

    fn initial(filename: String, source: String) -> HistoryEntry {
        HistoryEntry {
            stacks: vec![StackFrame {
                level: 0,
                source: SourceContext {
                    source,
                    filename,
                    line_no: 0,
                },
                context: None,
            }],
        }
    }

    pub fn source(&self, level: u16) -> SourceContext {
        let entry = self.stacks.get(level as usize);
        match entry {
            Some(e) => e.source.clone(),
            None => SourceContext::default(),
        }
    }

    pub(crate) fn stack(&self, stack_depth: u16) -> Option<&StackFrame> {
        self.stacks.get(stack_depth as usize)
    }
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

    pub(crate) fn current_mut(&mut self) -> Option<&mut HistoryEntry> {
        self.entries.get_mut(self.offset)
    }

    fn push(&mut self, entry: HistoryEntry) {
        self.entries.push(entry);
        self.offset = self.entries.len() - 1;
    }
}

#[derive(Clone, Debug)]
pub struct SourceContext {
    pub source: String,
    pub filename: String,
    pub line_no: u32,
}
impl SourceContext {
    fn default() -> SourceContext {
        SourceContext {
            source: "".to_string(),
            filename: "".to_string(),
            line_no: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub enum CurrentView {
    Listen,
    Session,
    Help,
}

pub struct App {
    pub is_connected: bool,
    pub notification: Notification,
    pub config: Config,
    receiver: Receiver<AppEvent>,
    quit: bool,
    sender: Sender<AppEvent>,

    pub server_status: Option<ContinuationStatus>,
    pub command_input: Input,
    pub command_response: Option<String>,
    pub client: Arc<Mutex<DbgpClient>>,

    pub history: History,

    pub view_current: CurrentView,
    pub session_view: SessionViewState,
    pub input_plurality: Vec<char>,

    pub counter: u16,

    pub snapshot_notify: Arc<Notify>,
    pub context_depth: u8,
    pub theme: Theme,

    pub analyzed_files: AnalyzedFiles,

    pub stack_max_context_fetch: u16,
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
            counter: 0,
            context_depth: 4,
            stack_max_context_fetch: 4,

            theme: Theme::SolarizedDark,
            server_status: None,
            command_input: Input::default(),
            command_response: None,
            view_current: CurrentView::Listen,
            session_view: SessionViewState::new(),

            snapshot_notify: Arc::new(Notify::new()),

            analyzed_files: HashMap::new(),
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

        self.notification = Notification::info("Welcome to debug-tui press ? for help".to_string());

        loop {
            let event = self.receiver.recv().await;

            if event.is_none() {
                continue;
            }

            let event = event.unwrap();

            if let Err(e) = self.handle_event(terminal, event).await {
                self.notification = Notification::error(e.to_string());
                continue;
            };

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
            AppEvent::Tick => (),
            _ => info!("Handling event {:?}", event),
        };
        match event {
            AppEvent::Tick => (),
            AppEvent::Quit => self.quit = true,
            AppEvent::ChangeView(view) => {
                self.view_current = view;
            }
            AppEvent::ChangeSessionViewMode(mode) => {
                self.session_view.mode = mode;
            }
            AppEvent::NextPane => {
                for _ in 0..self.take_motion() {
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
                for _ in 0..self.take_motion() {
                    self.history.next();
                    if self.history.is_current() && self.is_connected {
                        self.sender
                            .send(AppEvent::ChangeSessionViewMode(SessionViewMode::Current))
                            .await?;
                    }
                }
            }
            AppEvent::HistoryPrevious => {
                for _ in 0..self.take_motion() {
                    self.history.previous();
                }
            }
            AppEvent::ClientConnected(s) => {
                let mut client = self.client.lock().await;
                let response = client.deref_mut().connect(s).await?;
                for (feature, value) in [
                    ("max_depth", self.context_depth.to_string().as_str()),
                    ("extended_properties", "1"),
                ] {
                    info!("setting feature {} to {:?}", feature, value);
                    client.feature_set(feature, value).await?;
                }
                self.is_connected = true;
                self.server_status = None;
                self.view_current = CurrentView::Session;
                self.session_view.mode = SessionViewMode::Current;
                let source = client.source(response.fileuri.clone()).await.unwrap();

                self.history = History::default();
                self.history
                    .push(HistoryEntry::initial(response.fileuri.clone(), source));
            }
            AppEvent::Snapshot() => {
                self.snapshot().await?;
                self.snapshot_notify.notify_one();
            }
            AppEvent::StepOut => {
                self.exec_continuation(AppEvent::StepOut).await;
            }
            AppEvent::StepInto => {
                self.exec_continuation(AppEvent::StepInto).await;
            }
            AppEvent::StepOver => {
                self.exec_continuation(AppEvent::StepOver).await;
            }
            AppEvent::Run => {
                self.exec_continuation(AppEvent::Run).await;
            }
            AppEvent::ContextDepth(inc) => {
                let depth = self.context_depth as i8;
                self.context_depth = depth.wrapping_add(inc).max(0) as u8;
                self.client
                    .lock()
                    .await
                    .feature_set("max_depth", self.context_depth.to_string().as_str())
                    .await?;
            }
            AppEvent::ScrollSource(amount) => {
                self.session_view.source_scroll = apply_scroll(
                    self.session_view.source_scroll,
                    amount,
                    self.take_motion() as i16,
                );
            }
            AppEvent::ScrollContext(amount) => {
                self.session_view.context_scroll = apply_scroll(
                    self.session_view.context_scroll,
                    amount,
                    self.take_motion() as i16,
                );
            }
            AppEvent::ScrollStack(amount) => {
                self.session_view.stack_scroll = apply_scroll(
                    self.session_view.stack_scroll,
                    amount,
                    self.take_motion() as i16,
                );
                self.populate_stack_context().await?;
            }
            AppEvent::ToggleFullscreen => {
                self.session_view.full_screen = !self.session_view.full_screen;
            }
            AppEvent::UpdateStatus(server_status) => {
                if let ContinuationStatus::Stopping = server_status {
                    self.sender.send(AppEvent::Disconnect).await.unwrap();
                }
                self.server_status = Some(server_status);
            }
            AppEvent::Disconnect => {
                let _ = self.client.lock().await.deref_mut().disonnect().await;
                self.is_connected = false;
                self.sender
                    .send(AppEvent::ChangeSessionViewMode(SessionViewMode::History))
                    .await?;
            }
            AppEvent::PushInputPlurality(char) => self.input_plurality.push(char),
            AppEvent::Input(key_event) => match key_event.code {
                KeyCode::Char('t') => {
                    self.theme = self.theme.next();
                    self.notification =
                        Notification::info(format!("Switched to theme: {:?}", self.theme));
                }
                KeyCode::Char('?') => {
                    self.sender
                        .send(AppEvent::ChangeView(CurrentView::Help))
                        .await
                        .unwrap();
                }
                _ => self.send_event_to_current_view(event).await,
            },
            _ => self.send_event_to_current_view(event).await,
        };

        Ok(())
    }

    // generically handle "continuation" events and update the
    // application state accordingly.
    async fn exec_continuation(&mut self, event: AppEvent) {
        let client = Arc::clone(&self.client);
        let sender = self.sender.clone();
        let count = self.take_motion();

        let snapshot_notify = Arc::clone(&self.snapshot_notify);
        snapshot_notify.notify_one();

        tokio::spawn(async move {
            let mut last_response: Option<ContinuationResponse> = None;
            for i in 0..count {
                // we need to wait for the snapshot to complete before running a further
                // continuation.
                snapshot_notify.notified().await;

                info!("Running iteration {}/{}", i, count);
                let response = {
                    let mut instance = client.lock().await;
                    match event {
                        AppEvent::Run => instance.deref_mut().run().await,
                        AppEvent::StepOut => instance.deref_mut().step_out().await,
                        AppEvent::StepOver => instance.deref_mut().step_over().await,
                        AppEvent::StepInto => instance.deref_mut().step_into().await,
                        _ => panic!("Unexpected continuation event: {:?}", event),
                    }
                };

                match response {
                    Ok(response) => {
                        last_response = Some(response.clone());
                        match response.status {
                            ContinuationStatus::Break => {
                                sender.send(AppEvent::Snapshot()).await.unwrap();
                            }
                            ContinuationStatus::Stopping => {
                                break;
                            }
                            _ => (),
                        };
                        continue;
                    }
                    Err(_) => {
                        sender.send(AppEvent::Disconnect).await.unwrap();
                    }
                };
            }
            if let Some(last_response) = last_response {
                sender
                    .send(AppEvent::UpdateStatus(last_response.status))
                    .await
                    .unwrap();
            }
        });
    }

    // route the event to the currently selected view
    async fn send_event_to_current_view(&mut self, event: AppEvent) {
        let subsequent_event = match self.view_current {
            CurrentView::Help => HelpView::handle(self, event),
            CurrentView::Listen => ListenView::handle(self, event),
            CurrentView::Session => SessionView::handle(self, event),
        };
        if let Some(event) = subsequent_event {
            self.sender.send(event).await.unwrap()
        };
    }

    // take the current motion (i.e. number of times to repeat a command)
    pub fn take_motion(&mut self) -> u16 {
        if self.input_plurality.is_empty() {
            return 1;
        }
        let input = String::from_iter(&self.input_plurality);
        self.input_plurality = Vec::new();
        match input.parse::<u16>() {
            Ok(i) => i.min(255),
            Err(e) => {
                warn!("take_motion: {}", e);
                1
            }
        }
    }

    /// capture the current status and push it onto the history stack
    pub async fn snapshot(&mut self) -> Result<()> {
        let mut client = self.client.lock().await;
        let stack = client.deref_mut().get_stack().await?;
        let mut entry = HistoryEntry::new();
        for (level, frame) in stack.entries.iter().enumerate() {
            let filename = &frame.filename;
            let line_no = frame.line;
            let context = match (level as u16) < self.stack_max_context_fetch {
                true => Some(client.deref_mut().context_get(level as u16).await.unwrap()),
                false => None,
            };
            let source_code = client
                .deref_mut()
                .source(filename.to_string())
                .await
                .unwrap();

            let source = SourceContext {
                source: source_code,
                filename: filename.to_string(),
                line_no,
            };

            match self.analyzed_files.entry(filename.clone()) {
                Entry::Occupied(_) => (),
                Entry::Vacant(vacant_entry) => {
                    let mut analyser = Analyser::new();
                    vacant_entry.insert(analyser.analyze(source.source.as_str()).unwrap());
                }
            };

            entry.push(StackFrame {
                level: (level as u16),
                source,
                context,
            });
        }
        self.history.push(entry);
        self.session_view.reset();
        Ok(())
    }

    pub(crate) fn theme(&self) -> Scheme {
        self.theme.scheme()
    }

    async fn populate_stack_context(&mut self) -> Result<()> {
        if !self.history.is_current() {
            return Ok(());
        }
        let level = self.session_view.stack_scroll.0 as usize;
        if let Some(c) = self.history.current_mut() {
            let stack = c.stacks.get_mut(level);
            if let Some(s) = stack {
                if s.context.is_none() {
                    let mut client = self.client.lock().await;
                    let context = client.deref_mut().context_get(level as u16).await?;
                    s.context = Some(context);
                }
            };
        };
        Ok(())
    }
}

fn apply_scroll(scroll: (u16, u16), amount: (i16, i16), motion: i16) -> (u16, u16) {
    (
        (scroll.0 as i16).saturating_add(amount.0 * motion).max(0) as u16,
        (scroll.1 as i16).saturating_add(amount.1 * motion).max(0) as u16,
    )
}
