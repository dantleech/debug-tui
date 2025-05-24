use super::View;
use crate::app::App;
use crate::dbgp::client::Property;
use crate::dbgp::client::PropertyType;
use crate::event::input::AppEvent;
use crate::theme::Scheme;
use crossterm::event::KeyCode;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use tui_input::backend::crossterm::EventHandler;

pub struct ContextComponent {}

impl View for ContextComponent {
    fn handle(app: &mut App, event: AppEvent) -> Option<AppEvent> {
        if app.session_view.context_search.show {
            return match event {
                AppEvent::Input(e) => {
                    if e.code == KeyCode::Esc {
                        return Some(AppEvent::ContextSearchClose);
                    }
                    app.session_view.context_search.input.handle_event(&crossterm::event::Event::Key(e));
                    return None;
                },
                _ => None,
            }
        }
        match event {
            AppEvent::Scroll(scroll) => Some(AppEvent::ScrollContext(scroll)),
            AppEvent::Input(e) => {
                match e.code {
                    KeyCode::Char('/') => Some(AppEvent::ContextSearchOpen),
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
                if app.session_view.context_search.show { 1 } else { 0 }
            ), Constraint::Min(1)]);
        let areas = layout.split(area);

        frame.render_widget(Paragraph::new(app.session_view.context_search.input.value()), areas[0]);
            
        draw_properties(&app.theme(), &context.properties, &mut lines, 0);


        frame.render_widget(
            Paragraph::new(lines).scroll(app.session_view.context_scroll),
            areas[1],
        );
    }
}

pub fn draw_properties(
    theme: &Scheme,
    properties: &Vec<Property>,
    lines: &mut Vec<Line>,
    level: usize,
) {
    for property in properties {
        let mut spans = vec![
            Span::raw("  ".repeat(level)),
            Span::styled(property.name.to_string(), theme.syntax_label),
            Span::raw(" ".to_string()),
            Span::styled(
                property.type_name(),
                match property.property_type {
                    PropertyType::Object => theme.syntax_type_object,
                    _ => theme.syntax_type,
                },
            ),
            Span::raw(" = ".to_string()),
            render_value(theme, property),
        ];

        let delimiters = match property.property_type {
            PropertyType::Array => ("[", "]"),
            _ => ("{", "}"),
        };

        if !property.children.is_empty() {
            spans.push(Span::raw(delimiters.0).style(theme.syntax_brace));
        }

        lines.push(Line::from(spans));

        if !property.children.is_empty() {
            draw_properties(theme, &property.children, lines, level + 1);
            lines.push(Line::from(vec![Span::raw(delimiters.1)]).style(theme.syntax_brace));
        }
    }
}

pub fn render_value<'a>(theme: &Scheme, property: &Property) -> Span<'a> {
    let value = property.value.clone().unwrap_or("".to_string());
    match property.property_type {
        PropertyType::Bool => Span::styled(value, theme.syntax_literal),
        PropertyType::Int => Span::styled(value, theme.syntax_literal),
        PropertyType::Float => Span::styled(value, theme.syntax_literal),
        PropertyType::String => Span::styled(format!("\"{}\"", value), theme.syntax_literal_string),
        PropertyType::Array => Span::styled(value, theme.syntax_literal),
        PropertyType::Hash => Span::styled(value, theme.syntax_literal),
        PropertyType::Object => Span::styled(value, theme.syntax_literal),
        PropertyType::Resource => Span::styled(value, theme.syntax_literal),
        PropertyType::Undefined => Span::styled(value, theme.syntax_literal),
        _ => Span::styled(value, theme.syntax_literal),
    }
}
