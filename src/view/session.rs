use super::context::ContextComponent;
use super::eval::ChannelsComponent;
use super::eval::EvalState;
use super::source::SourceComponent;
use super::stack::StackComponent;
use super::Col;
use super::ComponentType;
use super::Pane;
use super::View;
use crate::app::App;
use crate::app::ListenStatus;
use crate::event::input::AppEvent;
use crossterm::event::KeyCode;
use crossterm::event::KeyModifiers;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::layout::Offset;
use ratatui::layout::Rect;
use ratatui::widgets::block::Title;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Clear;
use ratatui::widgets::Tabs;
use ratatui::Frame;
use std::cell::Cell;
use std::rc::Rc;

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

        let multiplier = if KeyModifiers::SHIFT == input_event.modifiers & KeyModifiers::SHIFT {
            10
        } else {
            1
        };

        // handle global session events
        match input_event.code {
            KeyCode::Tab => return Some(AppEvent::NextPane),
            KeyCode::BackTab => return Some(AppEvent::PreviousPane),
            KeyCode::Enter => return Some(AppEvent::ToggleFullscreen),
            KeyCode::Left => return Some(AppEvent::Scroll((0, -multiplier))),
            KeyCode::Right => return Some(AppEvent::Scroll((0, multiplier))),
            KeyCode::Up => return Some(AppEvent::Scroll((-multiplier, 0))),
            KeyCode::Down => return Some(AppEvent::Scroll((multiplier, 0))),
            KeyCode::Char(char) => match char {
                'e' => return Some(AppEvent::EvalStart),
                'c' => return Some(AppEvent::NextChannel),
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
                    'd' => Some(AppEvent::Disconnect),
                    _ => None,
                },
                _ => None,
            },
            SessionViewMode::History => match input_event.code {
                KeyCode::Esc => escape(app),
                KeyCode::Char(c) => match c {
                    'n' => Some(AppEvent::HistoryNext),
                    'p' => Some(AppEvent::HistoryPrevious),
                    'd' => Some(AppEvent::Disconnect),
                    'b' => escape(app),
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
        let mut pane_index = 0;

        let left_panes = app.session_view.panes(Col::Left);
        let left_rows = split_rows(&left_panes, cols[0]);
        for (row_index, pane) in left_panes.iter().enumerate() {
            build_pane_widget(frame, app, pane, left_rows[row_index], pane_index);
            pane_index += 1;
        }

        let right_panes = app.session_view.panes(Col::Right);
        let right_rows = split_rows(&right_panes, cols[1]);
        for (row_index, pane) in right_panes.iter().enumerate() {
            build_pane_widget(frame, app, pane, right_rows[row_index], pane_index);
            pane_index += 1;
        }
    }
}

fn escape(app: &App) -> Option<AppEvent> {
    if app.listening_status == ListenStatus::Refusing {
        Some(AppEvent::Listen)
    } else {
        Some(AppEvent::ChangeSessionViewMode(SessionViewMode::Current))
    }
}

fn split_rows(panes: &Vec<&Pane>, area: Rect) -> Rc<[Rect]> {
    let mut vertical_constraints = Vec::new();

    for pane in panes {
        vertical_constraints.push(pane.constraint);
    }

    Layout::vertical(vertical_constraints).split(area)
}

fn delegate_event_to_pane(app: &mut App, event: AppEvent) -> Option<AppEvent> {
    let focused_pane = app.session_view.current_pane();

    match focused_pane.component_type {
        ComponentType::Source => SourceComponent::handle(app, event),
        ComponentType::Context => ContextComponent::handle(app, event),
        ComponentType::Stack => StackComponent::handle(app, event),
        ComponentType::Eval => ChannelsComponent::handle(app, event),
    }
}

fn build_pane_widget(frame: &mut Frame, app: &App, pane: &Pane, area: Rect, index: usize) {
    let block = Block::default()
        .borders(Borders::all())
        .title_bottom(match pane.component_type {
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
                    "" => "press 'f' to filter with dot notation",
                    _ => app.session_view.context_filter.input.value(),
                }
            ),
            ComponentType::Stack => format!(
                "Stack({}/{}, fetch-depth: {})",
                app.session_view.stack_depth(),
                match app.history.current() {
                    Some(e) => e.stacks.len().saturating_sub(1),
                    None => 0,
                },
                app.stack_max_context_fetch,
            ),
            ComponentType::Eval => match app.history.current() {
                Some(entry) => format!(
                    "Eval: {} {}",
                    if let Some(eval) = &entry.eval {
                        eval.expr.clone()
                    } else {
                        "Press 'e' to enter an expression".to_string()
                    },
                    ", 'c' to change channel",
                ),
                None => "".to_string(),
            },
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
        ComponentType::Eval => {
            let tabs = Tabs::new(app.channels.names()).select(app.session_view.eval_state.channel);
            frame.render_widget(tabs, area.offset(Offset{x: 1, y: 0}));
            ChannelsComponent::draw(app, frame, block.inner(area));
        }
    };
}

#[derive(Default)]
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

#[derive(Default)]
pub struct SessionViewState {
    pub full_screen: bool,
    pub source_scroll: (u16, u16),
    pub source_area: Cell<Rect>,
    pub eval_state: EvalState,
    pub context_scroll: (u16, u16),
    pub context_filter: SearchState,
    pub stack_scroll: (u16, u16),
    pub mode: SessionViewMode,
    pub panes: Vec<Pane>,
    pub current_pane: usize,
}

impl SessionViewState {
    pub fn new() -> Self {
        Self {
            full_screen: false,
            source_scroll: (0, 0),
            source_area: Cell::new(Rect::new(0, 0, 0, 0)),
            context_scroll: (0, 0),
            eval_state: EvalState::default(),
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
                    constraint: ratatui::layout::Constraint::Percentage(75),
                    col: Col::Left,
                },
                Pane {
                    component_type: ComponentType::Eval,
                    constraint: ratatui::layout::Constraint::Fill(1),
                    col: Col::Left,
                },
                Pane {
                    component_type: ComponentType::Context,
                    constraint: ratatui::layout::Constraint::Percentage(75),
                    col: Col::Right,
                },
                Pane {
                    component_type: ComponentType::Stack,
                    constraint: ratatui::layout::Constraint::Fill(1),
                    col: Col::Right,
                },
            ],
        }
    }

    fn panes(&self, col: Col) -> Vec<&Pane> {
        self.panes.iter().filter(|p| p.col == col).collect()
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

    pub(crate) fn scroll_to_line(&mut self, line_no: u32) {
        let area = self.source_area.get();
        let mid_point = (area.height as u32).div_ceil(2);
        let offset = line_no.saturating_sub(mid_point);
        self.source_scroll.0 = offset as u16;
    }

    pub(crate) fn stack_level(&self) -> usize {
        self.stack_scroll.0 as usize
    }
}

#[derive(Debug, Clone, Default)]
pub enum SessionViewMode {
    #[default]
    Current,
    History,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn panes() {
        let mut view = SessionViewState::default();
        view.panes = vec![
            Pane {
                component_type: ComponentType::Stack,
                constraint: Constraint::Min(1),
                col: Col::Left,
            },
            Pane {
                component_type: ComponentType::Stack,
                constraint: Constraint::Min(1),
                col: Col::Right,
            },
            Pane {
                component_type: ComponentType::Stack,
                constraint: Constraint::Min(1),
                col: Col::Right,
            },
        ];
        assert_eq!(1, view.panes(Col::Left).len());
        assert_eq!(2, view.panes(Col::Right).len());
    }

    #[test]
    pub fn scroll_to_line() {
        let mut view = SessionViewState::default();
        view.source_area = Cell::new(Rect {
            x: 0,
            y: 0,
            width: 0,
            height: 10,
        });
        view.scroll_to_line(0);

        assert_eq!(0, view.source_scroll.0);

        view.scroll_to_line(5);
        assert_eq!(0, view.source_scroll.0);

        view.scroll_to_line(6);
        assert_eq!(1, view.source_scroll.0);

        view.scroll_to_line(10);
        assert_eq!(5, view.source_scroll.0);

        view.scroll_to_line(20);
        assert_eq!(15, view.source_scroll.0);

        view.scroll_to_line(100);
        assert_eq!(95, view.source_scroll.0);
    }
}
