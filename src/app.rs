use crate::analyzer::Analyser;
use crate::analyzer::Analysis;
use crate::analyzer::VariableRef;
use crate::channel::Channels;
use crate::config::Config;
use crate::dbgp::client::ContextGetResponse;
use crate::dbgp::client::ContinuationResponse;
use crate::dbgp::client::ContinuationStatus;
use crate::dbgp::client::DbgpClient;
use crate::dbgp::client::EvalResponse;
use crate::dbgp::client::Property;
use crate::event::input::AppEvent;
use crate::notification::Notification;
use crate::php_process;
use crate::php_process::ProcessEvent;
use crate::theme::Scheme;
use crate::theme::Theme;
use crate::view::eval::draw_properties;
use crate::view::eval::EvalDialog;
use crate::view::help::HelpView;
use crate::view::layout::LayoutView;
use crate::view::listen::ListenView;
use crate::view::session::SessionView;
use crate::view::session::SessionViewMode;
use crate::view::session::SessionViewState;
use crate::view::View;
use crate::workspace::Workspace;
use anyhow::Result;
use crossterm::event::KeyCode;
use log::error;
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
use tokio::sync::mpsc;
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
#[derive(Clone,Debug)]
pub struct Variable {
    pub var_ref: VariableRef,
    pub value: Property,
}
#[derive(Default)]
pub struct DocumentVariables {
    doclinemap: HashMap<String,Vec<Variable>>
}
impl DocumentVariables {
    pub fn put(&mut self, context: &SourceContext, variables: Vec<Variable>) {
        let entry = self.doclinemap.entry(format!("{}:{}", context.filename, context.line_no));
        entry.insert_entry(variables);
    }

    pub fn get(&self, source_file: &String, line_no: u32) -> Vec<Variable> {
        match self.doclinemap.get(&format!("{}:{}", source_file, line_no)) {
            Some(v) => v.to_vec(),
            None => vec![],
        }

    }
}
impl StackFrame {
    pub(crate) fn get_property(&self, name: &str) -> Option<&Property> {
        match &self.context {
            Some(c) => c.properties.get(name),
            None => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct HistoryEntry {
    pub stacks: Vec<StackFrame>,
    pub eval: Option<EvalEntry>,
}

#[derive(Clone, Debug)]
pub struct EvalEntry {
    pub expr: String,
    pub response: EvalResponse,
}

impl HistoryEntry {
    fn push(&mut self, frame: StackFrame) {
        self.stacks.push(frame);
    }
    fn new() -> Self {
        let stacks = Vec::new();
        HistoryEntry { stacks, eval: None }
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
            eval: None,
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
pub enum SelectedView {
    Listen,
    Session,
    Help,
}

#[derive(Debug, Clone)]
pub enum ActiveDialog {
    Eval,
}

#[derive(PartialEq)]
pub enum ListenStatus {
    Connected,
    Listening,
    Refusing,
}

impl ListenStatus {
    pub fn is_connected(&self) -> bool {
        *self == ListenStatus::Connected
    }
}

pub struct App {
    tick: u8,
    receiver: Receiver<AppEvent>,
    quit: bool,
    sender: Sender<AppEvent>,

    pub channels: Channels,
    pub listening_status: ListenStatus,
    pub notification: Notification,
    pub config: Config,

    pub server_status: Option<ContinuationStatus>,
    pub command_input: Input,
    pub command_response: Option<String>,
    pub client: Arc<Mutex<DbgpClient>>,
    pub workspace: Workspace,

    pub history: History,
    pub document_variables: DocumentVariables,

    pub view_current: SelectedView,
    pub focus_view: bool,
    pub session_view: SessionViewState,
    pub active_dialog: Option<ActiveDialog>,
    pub input_plurality: Vec<char>,

    pub counter: u16,

    pub snapshot_notify: Arc<Notify>,
    pub context_depth: u16,
    pub theme: Theme,

    pub analyzed_files: AnalyzedFiles,

    pub stack_max_context_fetch: u16,
    php_tx: Sender<ProcessEvent>,
}

impl App {
    pub fn new(config: Config, receiver: Receiver<AppEvent>, sender: Sender<AppEvent>) -> App {
        let client = Arc::new(Mutex::new(DbgpClient::new(None)));
        let (php_tx, php_rx) = mpsc::channel::<ProcessEvent>(1024);
        php_process::process_manager_start(php_rx, sender.clone());
        App {
            tick: 0,
            listening_status: ListenStatus::Listening,
            config,
            input_plurality: vec![],
            notification: Notification::none(),
            receiver,
            sender: sender.clone(),
            php_tx,
            quit: false,
            history: History::default(),
            document_variables: DocumentVariables::default(),
            client: Arc::clone(&client),
            workspace: Workspace::new(Arc::clone(&client)),
            channels: Channels::new(),

            counter: 0,
            context_depth: 4,
            stack_max_context_fetch: 1,

            theme: Theme::SolarizedDark,
            server_status: None,
            command_input: Input::default(),
            command_response: None,
            view_current: SelectedView::Listen,
            active_dialog: None,
            focus_view: false,
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

            sender.send(AppEvent::Listening).await.unwrap();

            loop {
                match listener.accept().await {
                    Ok(s) => match sender.send(AppEvent::ClientConnected(s.0)).await {
                        Ok(_) => (),
                        Err(e) => error!("Could not send connection event: {}", e),
                    },
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
                self.active_dialog = None;
                self.notification = Notification::error(e.to_string());
                continue;
            };

            if self.quit {
                return Ok(());
            }

            self.channels.unload().await;

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
            AppEvent::Tick => {
                self.tick = self.tick.wrapping_add(1);
            }
            _ => info!("Handling event {:?}", event),
        };
        match event {
            AppEvent::Tick => (),
            AppEvent::Quit => self.quit = true,
            AppEvent::Listening => {
                if let Some(script) = &self.config.cmd {
                    self.php_tx.send(ProcessEvent::Start(script.to_vec())).await?;
                }
            },
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
            AppEvent::PreviousPane => {
                for _ in 0..self.take_motion() {
                    self.session_view.prev_pane();
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
                    self.recenter();
                    if self.history.is_current()
                        && (self.listening_status == ListenStatus::Connected)
                    {
                        self.sender
                            .send(AppEvent::ChangeSessionViewMode(SessionViewMode::Current))
                            .await?;
                    }
                }
            }
            AppEvent::HistoryPrevious => {
                for _ in 0..self.take_motion() {
                    self.history.previous();
                    self.recenter();
                }
            }
            AppEvent::Listen => {
                self.listening_status = ListenStatus::Listening;
                self.view_current = SelectedView::Listen;
                self.session_view.mode = SessionViewMode::Current;
                self.notification = Notification::info("listening for next connection".to_string())
            }
            AppEvent::ClientConnected(s) => {
                if self.listening_status != ListenStatus::Listening {
                    self.notification =
                        Notification::warning("refused incoming connection".to_string());
                } else {
                    self.notification = Notification::info("connected".to_string());
                    let filepath = {
                        let mut client = self.client.lock().await;
                        let response = client.deref_mut().connect(s).await?;
                        for (feature, value) in [
                            ("max_depth", self.context_depth.to_string().as_str()),
                            ("extended_properties", "1"),
                        ] {
                            info!("setting feature {} to {:?}", feature, value);
                            client.feature_set(feature, value).await?;
                        }
                        response.fileuri.clone()
                    };
                    self.listening_status = ListenStatus::Connected;
                    self.reset();

                    let source = self.workspace.open(filepath.clone()).await;
                    self.history = History::default();
                    self.history
                        .push(HistoryEntry::initial(filepath.clone(), source.text.clone()));
                }
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
                let depth = self.context_depth;
                self.context_depth = depth.wrapping_add(inc as u16).clamp(1, 9);
                self.client
                    .lock()
                    .await
                    .feature_set("max_depth", self.context_depth.to_string().as_str())
                    .await?;
            }
            AppEvent::ContextFilterOpen => {
                self.session_view.context_filter.show = true;
                self.focus_view = true;
            }
            AppEvent::ContextSearchClose => {
                self.session_view.context_filter.show = false;
                self.focus_view = false;
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
            AppEvent::ScrollEval(amount) => {
                self.session_view.eval_state.scroll = apply_scroll(
                    self.session_view.eval_state.scroll,
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
                self.recenter();
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
                self.listening_status = ListenStatus::Refusing;
                self.sender
                    .send(AppEvent::ChangeSessionViewMode(SessionViewMode::History))
                    .await?;
            }
            AppEvent::PushInputPlurality(char) => self.input_plurality.push(char),
            AppEvent::EvalStart => {
                if !self.history.is_current() {
                    self.notification =
                        Notification::warning("Cannot eval in history mode".to_string());
                } else {
                    self.active_dialog = Some(ActiveDialog::Eval);
                }
            }
            AppEvent::NextChannel => {
                self.session_view.eval_state.channel = (self.session_view.eval_state.channel + 1) % self.channels.count()
            },
            AppEvent::FocusChannel(name) => {
                self.session_view.eval_state.focus(&self.channels, name);
            },
            AppEvent::NotifyError(message) => {
                self.notification = Notification::error(message);
            },
            AppEvent::ChannelLog(channel, chunk) => {
                let buffer = self.channels.get_mut(channel.as_str()).buffer.clone();
                buffer.lock().await.push_str(&chunk);
                self.sender
                    .send(AppEvent::FocusChannel(channel))
                    .await
                    .unwrap_or_default();
            },
            AppEvent::RestartProcess => {
                self.sender.send(AppEvent::Disconnect).await?;
                self.sender.send(AppEvent::Listen).await?;
                self.php_tx.send(ProcessEvent::Stop).await?;
                if let Some(cmd) = self.config.clone().cmd {
                    self.php_tx.send(ProcessEvent::Start(cmd)).await?;
                };
            },
            AppEvent::EvalCancel => {
                self.active_dialog = None;
            }
            AppEvent::EvalExecute => {
                if self.session_view.eval_state.input.to_string().is_empty() {
                    self.session_view.eval_state.response = None;
                } else {
                    let response = self
                        .client
                        .lock()
                        .await
                        .eval(
                            self.session_view.eval_state.input.to_string(),
                            self.session_view.stack_depth(),
                        )
                        .await?;

                    let mut lines: Vec<String> = Vec::new();
                    match response.error {
                        Some(e) => {
                            self.channels.get_mut("eval").writeln(
                                format!("[{}] {}", e.code, e.message),
                            ).await;
                        },
                        None => {
                            draw_properties(
                                response.properties.defined_properties(),
                                &mut lines,
                                0
                            );
                            self.channels.get_mut("eval").writeln(lines.join("\n")).await;
                        }
                    };
                    self.sender.send(AppEvent::FocusChannel("eval".to_string())).await.unwrap();
                }
                self.active_dialog = None;
            }
            AppEvent::Input(key_event) => {
                if self.active_dialog.is_some() {
                    self.send_event_to_current_dialog(event).await;
                } else if self.focus_view {
                    // event shandled exclusively by view (e.g. input needs focus)
                    self.send_event_to_current_view(event).await;
                } else {
                    // global events
                    match key_event.code {
                        KeyCode::Char('t') => {
                            self.theme = self.theme.next();
                            self.notification =
                                Notification::info(format!("Switched to theme: {:?}", self.theme));
                        }
                        KeyCode::Char('?') => {
                            self.sender
                                .send(AppEvent::ChangeView(SelectedView::Help))
                                .await
                                .unwrap();
                        }
                        _ => self.send_event_to_current_view(event).await,
                    }
                }
            }
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

    async fn send_event_to_current_dialog(&mut self, event: AppEvent) {
        if let Some(dialog) = &self.active_dialog {
            let subsequent_event = match &dialog {
                ActiveDialog::Eval => EvalDialog::handle(self, event),
            };
            if let Some(event) = subsequent_event {
                self.sender.send(event).await.unwrap()
            };
        }
    }

    // route the event to the currently selected view
    async fn send_event_to_current_view(&mut self, event: AppEvent) {
        let subsequent_event = match self.view_current {
            SelectedView::Help => HelpView::handle(self, event),
            SelectedView::Listen => ListenView::handle(self, event),
            SelectedView::Session => SessionView::handle(self, event),
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
        let mut entry = HistoryEntry::new();

        // for each stack frame fetch the context and analyse the source code
        let stack = { self.client.lock().await.deref_mut().get_stack().await? };
        for (level, frame) in stack.entries.iter().enumerate() {
            let filename = &frame.filename;
            let line_no = frame.line;
            let context = match (level as u16) < self.stack_max_context_fetch {
                true => Some(
                    self.client
                        .lock()
                        .await
                        .deref_mut()
                        .context_get(level as u16)
                        .await?,
                ),
                false => None,
            };

            let document = self.workspace.open(filename.to_string()).await;
            let source = SourceContext {
                source: document.text.to_string(),
                filename: document.filename.to_string(),
                line_no,
            };

            match self.analyzed_files.entry(filename.clone()) {
                Entry::Occupied(_) => (),
                Entry::Vacant(vacant_entry) => {
                    let mut analyser = Analyser::new();
                    vacant_entry.insert(analyser.analyze(source.source.as_str()).unwrap());
                }
            };

            let analysis = self.analyzed_files.get(&filename.clone());
            let stack = StackFrame {
                level: (level as u16),
                source,
                context,
            };

            // populate inline variables with values
            {
                let mut vars = vec![];
                if let Some(analysis) = analysis {
                    for (_, var) in analysis.row((line_no as usize).saturating_sub(1)) {
                        let property = stack.get_property(var.name.as_str());
                        if let Some(property) = property {
                            vars.push(Variable{ var_ref: var, value: property.clone() });
                        }
                    }

                    self.document_variables.put(&stack.source, vars);
                }
            }

            entry.push(stack);
        }

        // *xdebug* only evalutes expressions on the current stack frame
        let eval = if !self.session_view.eval_state.input.to_string().is_empty() {
            let response = self
                .client
                .lock()
                .await
                .eval(
                    self.session_view.eval_state.input.to_string(),
                    self.session_view.stack_depth(),
                )
                .await?;

                Some(EvalEntry{
                    expr: self.session_view.eval_state.input.to_string(),
                    response
                })
        } else {
            None
        };

        entry.eval = eval;

        self.session_view.reset();
        self.history.push(entry);
        self.recenter();
        Ok(())
    }

    pub(crate) fn theme(&self) -> Scheme {
        self.theme.scheme()
    }

    async fn populate_stack_context(&mut self) -> Result<()> {
        if !self.history.is_current() {
            return Ok(());
        }
        let level = self.session_view.stack_level();
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

    fn reset(&mut self) {
        self.server_status = None;
        self.view_current = SelectedView::Session;
        self.session_view.mode = SessionViewMode::Current;
        self.analyzed_files = HashMap::new();
        self.workspace.reset();
    }

    fn recenter(&mut self) {
        let entry = self.history.current();
        if let Some(entry) = entry {
            self.session_view
                .scroll_to_line(entry.source(self.session_view.stack_depth()).line_no)
        }
    }
}

fn apply_scroll(scroll: (u16, u16), amount: (i16, i16), motion: i16) -> (u16, u16) {
    (
        (scroll.0 as i16).saturating_add(amount.0 * motion).max(0) as u16,
        (scroll.1 as i16).saturating_add(amount.1 * motion).max(0) as u16,
    )
}
