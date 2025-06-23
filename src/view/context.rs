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

pub fn draw_properties(
    theme: &Scheme,
    properties: &Vec<Property>,
    lines: &mut Vec<Line>,
    level: usize,
    filter_path: &mut Vec<&str>,
) {
    let filter = filter_path.pop();

    for property in properties {
        if let Some(filter) = filter {
            if !property.name.starts_with(filter) {
                continue;
            }
        }
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
            draw_properties(theme, &property.children, lines, level + 1, filter_path);
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

#[cfg(test)]
mod test {
    use crate::theme::Theme;
    use anyhow::Result;

    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_draw_properties_empty() -> Result<()> {
        let mut lines = vec![];
        draw_properties(
            &Theme::SolarizedDark.scheme(),
            &Vec::new(),
            &mut lines,
            0,
            &mut Vec::new(),
        );
        assert_eq!(0, lines.len());
        Ok(())
    }

    #[test]
    fn test_draw_properties_two_levels() -> Result<()> {
        let mut lines = vec![];
        let mut prop1 = Property::default();
        let mut prop2 = Property::default();
        prop2.name = "bar".to_string();
        prop1.children = vec![
            prop2
        ];
        prop1.name = "foo".to_string();

        draw_properties(
            &Theme::SolarizedDark.scheme(),
            &vec![
                prop1
            ],
            &mut lines,
            0,
            &mut Vec::new(),
        );
        assert_eq!(vec![
            "foo string = \"\"{",
            "  bar string = \"\"",
            "}",
        ], lines.iter().map(
            |l| { l.to_string()}
        ).collect::<Vec<String>>());
        Ok(())
    }

    #[test]
    fn test_filter_property_multiple_level() -> Result<()> {
        let mut lines = vec![];
        let mut prop1 = Property::default();
        let mut prop2 = Property::default();
        let prop3 = Property::default();

        prop2.name = "bar".to_string();
        prop1.children = vec![
            prop2
        ];
        prop1.name = "foo".to_string();

        // segments are reversed
        let mut filter = &mut vec![
            "bar",
            "foo",
        ];

        draw_properties(
            &Theme::SolarizedDark.scheme(),
            &vec![
                prop1,
                prop3
            ],
            &mut lines,
            0,
            filter,
        );

        assert_eq!(vec![
            "foo string = \"\"{",
            "  bar string = \"\"",
            "}",
        ], lines.iter().map(
            |l| { l.to_string()}
        ).collect::<Vec<String>>());

        Ok(())
    }
}
