use super::properties::draw_properties;
use super::View;
use crate::app::App;
use crate::event::input::AppEvent;
use crossterm::event::KeyCode;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use tui_input::backend::crossterm::EventHandler;

pub struct ContextComponent {}

impl View for ContextComponent {
    fn handle(app: &mut App, event: AppEvent) -> Option<AppEvent> {
        if app.session_view.context_filter.show {
            return match event {
                AppEvent::Input(e) => {
                    if e.code == KeyCode::Esc {
                        return Some(AppEvent::ContextSearchClose);
                    }
                    if e.code == KeyCode::Enter {
                        return Some(AppEvent::ContextSearchClose);
                    }
                    app.session_view.context_filter.input.handle_event(&crossterm::event::Event::Key(e));
                    return None;
                },
                _ => None,
            }
        }
        match event {
            AppEvent::Scroll(scroll) => Some(AppEvent::ScrollContext(scroll)),
            AppEvent::Input(e) => {
                match e.code {
                    KeyCode::Char('f') => Some(AppEvent::ContextFilterOpen),
                    _ => None,
                }
            },
            _ => None,
        }
    }

    fn draw(app: &App, frame: &mut Frame, area: Rect) {
        let entry = match app.history.current() {
            Some(e) => e,
            None => return,
        };
        let context = match entry.stack(app.session_view.stack_depth()) {
            Some(stack) => match &stack.context {
                Some(context) => context,
                None => return,
            },
            None => return,
        };
        let mut lines: Vec<Line> = vec![];
        let layout = Layout::default()
            .constraints([Constraint::Length(
                if app.session_view.context_filter.show { 3 } else { 0 }
            ), Constraint::Min(1)]);
        let areas = layout.split(area);

        frame.render_widget(Paragraph::new(Line::from(vec![
            Span::raw(app.session_view.context_filter.input.value()),
            Span::raw(" ").style(app.theme().cursor),  
        ])
        ).block(Block::default().borders(Borders::all())), areas[0]);
            
        let mut filter_path = app.session_view.context_filter.segments().clone();
        draw_properties(
            &app.theme(),
            &context.properties,
            &mut lines,
            0,
            &mut filter_path,
        );


        frame.render_widget(
            Paragraph::new(lines).scroll(app.session_view.context_scroll),
            areas[1],
        );
    }
}
