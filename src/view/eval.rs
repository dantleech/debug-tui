use super::centered_rect_absolute;
use super::View;
use crate::app::App;
use crate::dbgp::client::EvalResponse;
use crate::dbgp::client::Property;
use crate::dbgp::client::PropertyType;
use crate::event::input::AppEvent;
use crate::theme::Scheme;
use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Clear;
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use tui_input::backend::crossterm::EventHandler;
use tui_input::Input;

pub struct EvalComponent {}
pub struct EvalDialog {}

#[derive(Default)]
pub struct EvalState {
    pub response: Option<EvalResponse>,
    pub input: Input,
    pub scroll: (u16, u16),
}

impl View for EvalComponent {
    fn handle(_app: &mut App, event: AppEvent) -> Option<AppEvent> {
        match event {
            AppEvent::Scroll(scroll) => Some(AppEvent::ScrollEval(scroll)),
            _ => None,
        }
    }

    fn draw(app: &App, frame: &mut Frame, area: Rect) {
        if let Some(response) = &app.session_view.eval_state.response {
            if let Some(error) = &response.error {
                frame.render_widget(
                    Paragraph::new(error.message.clone()).style(app.theme().notification_error),
                    area,
                );
            } else {
                let mut lines: Vec<Line> = Vec::new();
                draw_properties(
                    &app.theme(),
                    &response.properties,
                    &mut lines,
                    0,
                    &mut Vec::new(),
                );
                frame.render_widget(
                    Paragraph::new(lines).scroll(app.session_view.eval_state.scroll),
                    area,
                );
            }
        }
    }
}

impl View for EvalDialog {
    fn handle(app: &mut App, event: AppEvent) -> Option<AppEvent> {
        match event {
            AppEvent::Input(e) => {
                if e.code == KeyCode::Esc {
                    return Some(AppEvent::EvalCancel);
                }
                if e.code == KeyCode::Enter {
                    return Some(AppEvent::EvalExecute);
                }
                app.session_view
                    .eval_state
                    .input
                    .handle_event(&crossterm::event::Event::Key(e));
                None
            }
            _ => None,
        }
    }

    fn draw(app: &App, frame: &mut Frame, area: Rect) {
        let darea = centered_rect_absolute(area.width - 10, 3, area);
        frame.render_widget(Clear, darea);
        frame.render_widget(
            Paragraph::new(Line::from(vec![Span::raw(
                app.session_view.eval_state.input.value(),
            )
            .style(app.theme().text_input)]))
            .block(
                Block::default()
                    .borders(Borders::all())
                    .title("Enter expression")
                    .style(app.theme().pane_border_active),
            ),
            darea,
        );

        let width = darea.width.max(3);
        let scroll = app
            .session_view
            .eval_state
            .input
            .visual_scroll(width as usize);
        let x = app
            .session_view
            .eval_state
            .input
            .visual_cursor()
            .max(scroll)
            - scroll
            + 1;
        frame.set_cursor_position((darea.x + x as u16, darea.y + 1));
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
    use super::*;
    use crate::theme::Theme;
    use anyhow::Result;
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
        prop1.children = vec![prop2];
        prop1.name = "foo".to_string();

        draw_properties(
            &Theme::SolarizedDark.scheme(),
            &vec![prop1],
            &mut lines,
            0,
            &mut Vec::new(),
        );
        assert_eq!(
            vec!["foo string = \"\"{", "  bar string = \"\"", "}",],
            lines
                .iter()
                .map(|l| { l.to_string() })
                .collect::<Vec<String>>()
        );
        Ok(())
    }

    #[test]
    fn test_filter_property_multiple_level() -> Result<()> {
        let mut lines = vec![];
        let mut prop1 = Property::default();
        let mut prop2 = Property::default();
        let prop3 = Property::default();

        prop2.name = "bar".to_string();
        prop1.children = vec![prop2];
        prop1.name = "foo".to_string();

        // segments are reversed
        let filter = &mut vec!["bar", "foo"];

        draw_properties(
            &Theme::SolarizedDark.scheme(),
            &vec![prop1, prop3],
            &mut lines,
            0,
            filter,
        );

        assert_eq!(
            vec!["foo string = \"\"{", "  bar string = \"\"", "}",],
            lines
                .iter()
                .map(|l| { l.to_string() })
                .collect::<Vec<String>>()
        );

        Ok(())
    }
}
