use super::context::ContextComponent;
use super::source::SourceComponent;
use super::stack::StackComponent;
use super::ComponentType;
use super::Pane;
use super::View;
use crate::app::App;
use crate::app::CurrentView;
use crate::app::InputMode;
use crate::event::input::AppEvent;
use crate::event::input::AppEvents;
use crossterm::event::KeyCode;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::Frame;

pub struct SessionView {}

impl View for SessionView {
    fn handle(app: &App, event: AppEvent) -> AppEvents {
        let input_event = match event {
            AppEvent::Input(key_event) => key_event,
            _ => return delegate_event_to_pane(app, event),
        };

        match app.input_mode {
            InputMode::Normal => (),
            _ => return AppEvents::none(),
        };

        // handle global session events
        match input_event.code {
            KeyCode::Tab => return AppEvents::one(AppEvent::NextPane),
            KeyCode::Enter => return AppEvents::one(AppEvent::ToggleFullscreen),
            KeyCode::Char(char) => match char {
                'j' => return AppEvents::one(AppEvent::ScrollDown(app.take_input_plurality().into())),
                'k' => return AppEvents::one(AppEvent::ScrollUp(1)),
                'J' => return AppEvents::one(AppEvent::ScrollDown(10)),
                'K' => return AppEvents::one(AppEvent::ScrollUp(10)),
                '0'..'9' => return AppEvents::one(AppEvent::PushInputPlurality(char)),
                _ => (),
            },
            _ => (),
        };

        let next_events: AppEvents = match app.session_view.mode {
            SessionViewMode::Current => match input_event.code {
                KeyCode::Char(char) => match char {
                    'r' => AppEvents::one(AppEvent::Run),
                    'n' => AppEvents::one(AppEvent::StepInto),
                    'N' => AppEvents::one(AppEvent::StepOver),
                    'o' => AppEvents::one(AppEvent::StepOut),
                    'p' => AppEvents::one(AppEvent::ChangeSessionViewMode(SessionViewMode::History)),
                    _ => AppEvents::none(),
                },
                _ => AppEvents::none(),
            },
            SessionViewMode::History => match input_event.code {
                KeyCode::Esc => AppEvents::one(AppEvent::ChangeView(CurrentView::Session)),
                KeyCode::Char(c) => match c {
                    'n' => AppEvents::one(AppEvent::HistoryNext),
                    'p' => AppEvents::one(AppEvent::HistoryPrevious),
                    'b' => AppEvents::one(AppEvent::ChangeSessionViewMode(SessionViewMode::Current)),
                    _ => AppEvents::none(),
                },
                _ => AppEvents::none(),
            },
        };

        if next_events.len() > 0 {
            return next_events;
        }

        delegate_event_to_pane(app, event)
    }

    fn draw(app: &App, frame: &mut Frame, area: ratatui::prelude::Rect) {
        if app.session_view.full_screen {
            build_pane_widget(frame, app, app.session_view.current_pane(), area, app.session_view.current_pane);
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

        let mut row_index = 0;
        for pane in &app.session_view.panes[1..] {
            build_pane_widget(frame, app, pane, rows[row_index], row_index + 1);
            row_index += 1;
        }
    }
}

fn delegate_event_to_pane(app: &App, event: AppEvent) -> AppEvents {
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
                Some(c) => c.source.filename.to_string(),
                None => "".to_string(),
            },
            ComponentType::Context => "Context".to_string(),
            ComponentType::Stack => "Stack".to_string(),
        })
        .style(
            Style::default().fg(if index == app.session_view.current_pane {
                Color::Green
            } else {
                Color::DarkGray
            }),
        );

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

pub struct SessionViewState {
    pub full_screen: bool,
    pub source_scroll: u16,
    pub context_scroll: u16,
    pub stack_scroll: u16,
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
            source_scroll: 0,
            context_scroll: 0,
            stack_scroll: 0,
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

    fn current_pane(&self) -> &Pane {
        self.panes.get(self.current_pane).unwrap()
    }

    pub(crate) fn reset(&mut self) {
        self.context_scroll = 0;
        self.source_scroll = 0;
    }
}

#[derive(Debug, Clone)]
pub enum SessionViewMode {
    Current,
    History,
}
