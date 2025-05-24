use super::context::ContextComponent;
use super::source::SourceComponent;
use super::stack::StackComponent;
use super::ComponentType;
use super::Pane;
use super::View;
use crate::app::App;
use crate::app::SelectedView;
use crate::event::input::AppEvent;
use crossterm::event::KeyCode;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Clear;
use ratatui::Frame;

pub struct SessionView {}

impl View for SessionView {
    fn handle(app: &mut App, event: AppEvent) -> Option<AppEvent> {
        let input_event = match event {
            AppEvent::Input(key_event) => key_event,
            _ => return delegate_event_to_pane(app, event),
        };

        if app.focus_view {
            return delegate_event_to_pane(app, event);
        }

        // handle global session events
        match input_event.code {
            KeyCode::Tab => return Some(AppEvent::NextPane),
            KeyCode::BackTab => return Some(AppEvent::PreviousPane),
            KeyCode::Enter => return Some(AppEvent::ToggleFullscreen),
            KeyCode::Char(char) => match char {
                'j' => return Some(AppEvent::Scroll((1, 0))),
                'k' => return Some(AppEvent::Scroll((-1, 0))),
                'J' => return Some(AppEvent::Scroll((10, 0))),
                'K' => return Some(AppEvent::Scroll((-10, 0))),
                'l' => return Some(AppEvent::Scroll((0, 1))),
                'L' => return Some(AppEvent::Scroll((0, 10))),
                'h' => return Some(AppEvent::Scroll((0, -1))),
                'H' => return Some(AppEvent::Scroll((0, -10))),
                '0'..='9' => return Some(AppEvent::PushInputPlurality(char)),
                _ => (),
            },
            _ => (),
        };

        let next_event: Option<AppEvent> = match app.session_view.mode {
            SessionViewMode::Current => match input_event.code {
                KeyCode::Char(char) => match char {
                    '+' => Some(AppEvent::ContextDepth(1)),
                    '-' => Some(AppEvent::ContextDepth(-1)),
                    'r' => Some(AppEvent::Run),
                    'n' => Some(AppEvent::StepInto),
                    'N' => Some(AppEvent::StepOver),
                    'o' => Some(AppEvent::StepOut),
                    'p' => Some(AppEvent::ChangeSessionViewMode(SessionViewMode::History)),
                    _ => None,
                },
                _ => None,
            },
            SessionViewMode::History => match input_event.code {
                KeyCode::Esc => Some(AppEvent::ChangeView(SelectedView::Session)),
                KeyCode::Char(c) => match c {
                    'n' => Some(AppEvent::HistoryNext),
                    'p' => Some(AppEvent::HistoryPrevious),
                    'b' => Some(AppEvent::ChangeSessionViewMode(SessionViewMode::Current)),
                    _ => None,
                },
                _ => None,
            },
        };

        if next_event.is_some() {
            return next_event;
        }

        delegate_event_to_pane(app, event)
    }

    fn draw(app: &App, frame: &mut Frame, area: ratatui::prelude::Rect) {
        if app.session_view.full_screen {
            build_pane_widget(
                frame,
                app,
                app.session_view.current_pane(),
                area,
                app.session_view.current_pane,
            );
            return;
        }

        let main_pane = match app.session_view.panes.first() {
            Some(pane) => pane,
            None => return,
        };

        let cols = Layout::horizontal(vec![main_pane.constraint, Constraint::Min(1)]).split(area);

        build_pane_widget(frame, app, main_pane, cols[0], 0);

        let mut vertical_constraints = Vec::new();

        for pane in &app.session_view.panes[1..] {
            vertical_constraints.push(pane.constraint);
        }

        let rows = Layout::vertical(vertical_constraints).split(cols[1]);

        for (row_index, pane) in app.session_view.panes[1..].iter().enumerate() {
            build_pane_widget(frame, app, pane, rows[row_index], row_index + 1);
        }
    }
}

fn delegate_event_to_pane(app: &mut App, event: AppEvent) -> Option<AppEvent> {
    let focused_pane = app.session_view.current_pane();

    match focused_pane.component_type {
        ComponentType::Source => SourceComponent::handle(app, event),
        ComponentType::Context => ContextComponent::handle(app, event),
        ComponentType::Stack => StackComponent::handle(app, event),
    }
}

fn build_pane_widget(frame: &mut Frame, app: &App, pane: &Pane, area: Rect, index: usize) {
    let block = Block::default()
        .borders(Borders::all())
        .title(match pane.component_type {
            ComponentType::Source => match app.history.current() {
                Some(c) => c
                    .source(app.session_view.stack_depth())
                    .filename
                    .to_string(),
                None => "".to_string(),
            },
            ComponentType::Context => format!(
                "Context(fetch-depth: {}, filter: {})",
                app.context_depth,
                match app.session_view.context_filter.input.value() {
                    "" => "n/a",
                    _ => app.session_view.context_filter.input.value(),
                }
            ),
            ComponentType::Stack => format!(
                "Stack({}/{}, fetch-depth: {})",
                app.session_view.stack_depth(),
                match app.history.current() {
                    Some(e) => e.stacks.len() - 1,
                    None => 0,
                },
                app.stack_max_context_fetch,
            ),
        })
        .style(match index == app.session_view.current_pane {
            true => app.theme().pane_border_active,
            false => app.theme().pane_border_inactive,
        });

    frame.render_widget(Clear, area);
    frame.render_widget(&block, area);

    match pane.component_type {
        ComponentType::Source => {
            SourceComponent::draw(app, frame, block.inner(area));
        }
        ComponentType::Context => {
            ContextComponent::draw(app, frame, block.inner(area));
        }
        ComponentType::Stack => {
            StackComponent::draw(app, frame, block.inner(area));
        }
    };
}

pub struct SearchState {
    pub show: bool,
    pub search: String,
    pub input: tui_input::Input,
}

impl SearchState {
    pub(crate) fn segments(&self) -> Vec<&str> {
        self.input.value().rsplit(".").collect()
    }
}

pub struct SessionViewState {
    pub full_screen: bool,
    pub source_scroll: (u16, u16),
    pub context_scroll: (u16, u16),
    pub context_filter: SearchState,
    pub stack_scroll: (u16, u16),
    pub mode: SessionViewMode,
    pub panes: Vec<Pane>,
    pub current_pane: usize,
}

impl Default for SessionViewState {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionViewState {
    pub fn new() -> Self {
        Self {
            full_screen: false,
            source_scroll: (0, 0),
            context_scroll: (0, 0),
            context_filter: SearchState {
                show: false,
                search: String::new(),
                input: tui_input::Input::default(),
            },
            stack_scroll: (0, 0),
            current_pane: 0,
            mode: SessionViewMode::Current,
            panes: vec![
                Pane {
                    component_type: ComponentType::Source,
                    constraint: ratatui::layout::Constraint::Percentage(70),
                },
                Pane {
                    component_type: ComponentType::Context,
                    constraint: ratatui::layout::Constraint::Percentage(70),
                },
                Pane {
                    component_type: ComponentType::Stack,
                    constraint: ratatui::layout::Constraint::Min(1),
                },
            ],
        }
    }

    pub fn next_pane(&mut self) {
        let next = self.current_pane + 1;
        self.current_pane = next % self.panes.len();
    }

    pub(crate) fn prev_pane(&mut self) {
        if self.current_pane == 0 {
            self.current_pane = self.panes.len() - 1;
        } else {
            let next = self.current_pane - 1;
            self.current_pane = next % self.panes.len();
        }
    }

    fn current_pane(&self) -> &Pane {
        self.panes.get(self.current_pane).unwrap()
    }

    pub(crate) fn reset(&mut self) {
        self.context_scroll = (0, 0);
        self.stack_scroll = (0, 0);
        self.source_scroll = (0, 0);
    }

    pub(crate) fn stack_depth(&self) -> u16 {
        self.stack_scroll.0
    }
}

#[derive(Debug, Clone)]
pub enum SessionViewMode {
    Current,
    History,
}
