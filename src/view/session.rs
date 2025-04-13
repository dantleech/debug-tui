use super::context;
use super::context::ContextComponent;
use super::source;
use super::source::SourceComponent;
use super::ComponentType;
use super::Pane;
use super::View;
use crate::app::App;
use crate::app::CurrentView;
use crate::app::InputMode;
use crate::event::input::AppEvent;
use crossterm::event::KeyCode;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::style::Stylize;
use ratatui::widgets::block::Title;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::Frame;

pub struct SessionView {}

impl View for SessionView {
    fn handle(app: &App, event: AppEvent) -> Option<AppEvent> {
        let input_event = match event {
            AppEvent::Input(key_event) => key_event,
            _ => return None,
        };

        match app.input_mode {
            InputMode::Normal => (),
            _ => return None,
        };

        // handle global session events
        match input_event.code {
            KeyCode::Tab => return Some(AppEvent::NextPane),
            KeyCode::Char(char) => match char {
                'j' => return Some(AppEvent::ScrollDown),
                'k' => return Some(AppEvent::ScrollUp),
                _ => (),
            },
            _ => (),
        };

        let next_event: Option<AppEvent> = match app.session_view.mode {
            SessionViewMode::Current => match input_event.code {
                KeyCode::Char(char) => match char {
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
                KeyCode::Esc => Some(AppEvent::ChangeView(CurrentView::Session)),
                KeyCode::Char(c) => match c {
                    'n' => Some(AppEvent::HistoryNext),
                    'p' => Some(AppEvent::HistoryPrevious),
                    'b' => Some(AppEvent::ChangeSessionViewMode(SessionViewMode::Current)),
                    _ => None,
                },
                _ => None,
            },
        };

        let focused_pane = app.session_view.current_pane();

        match focused_pane.component_type {
            ComponentType::Source => SourceComponent::handle(app, event),
            ComponentType::Context => ContextComponent::handle(app, event),
        }
    }

    fn draw(app: &App, frame: &mut Frame, area: ratatui::prelude::Rect) {
        let main_pane = match app.session_view.panes.get(0) {
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
            build_pane_widget(frame, app, &pane, rows[row_index], row_index + 1);
            row_index += 1;
        }
    }
}

fn build_pane_widget(frame: &mut Frame, app: &App, pane: &Pane, area: Rect, index: usize) -> () {
    let block = Block::default().borders(Borders::all()).style(Style::default().fg(
        if index == app.session_view.current_pane {
            Color::Green
        } else {
            Color::Gray
        }
    ));

    frame.render_widget(&block, area);

    match pane.component_type {
        ComponentType::Source => {
            SourceComponent::draw(app, frame, block.inner(area));
        }
        ComponentType::Context => {
            ContextComponent::draw(app, frame, block.inner(area));
        }
    };
}

pub struct SessionViewState {
    pub mode: SessionViewMode,
    pub panes: Vec<Pane>,
    pub current_pane: usize,
}

impl SessionViewState {
    pub fn new() -> Self {
        Self {
            current_pane: 0,
            mode: SessionViewMode::Current,
            panes: vec![
                Pane {
                    component_type: ComponentType::Source,
                    constraint: ratatui::layout::Constraint::Percentage(70),
                },
                Pane {
                    component_type: ComponentType::Context,
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
        return self.panes.get(self.current_pane).unwrap();
    }
}

#[derive(Debug)]
pub enum SessionViewMode {
    Current,
    History,
}
