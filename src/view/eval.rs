use std::cell::Cell;

use super::centered_rect_absolute;
use super::View;
use ansi_to_tui::IntoText;
use ratatui::layout::Offset;
use ratatui::widgets::Tabs;
use crate::app::App;
use crate::channel::Channel;
use crate::channel::Channels;
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

pub struct ChannelsComponent {}
pub struct EvalDialog {}

#[derive(Default)]
pub struct EvalState {
    pub response: Option<EvalResponse>,
    pub eval_area: Cell<Rect>,
    pub input: Input,
    pub channel: usize,
    pub scroll: (u16, u16),
}
impl EvalState {
    pub(crate) fn focus(&mut self, channels: &Channels, name: String) {
        self.channel = channels.offset_by_name(name).unwrap_or(0);
        self.scroll.1 = 0;
        let channel = channels.channel_by_offset(self.channel).expect("channel does not exist!");
        let area = self.eval_area.get();
        self.scroll.0 = (channel.lines.len() as i16 - area.height as i16).max(0) as u16;
    }
}

impl View for ChannelsComponent {
    fn handle(_app: &mut App, event: AppEvent) -> Option<AppEvent> {
        match event {
            AppEvent::Scroll(scroll) => Some(AppEvent::ScrollEval(scroll)),
            _ => None,
        }
    }

    fn draw(app: &App, frame: &mut Frame, inner_area: Rect, area: Rect) {
        let tabs = Tabs::new(app.channels.names()).select(app.session_view.eval_state.channel);
        frame.render_widget(tabs, area.offset(Offset{x: 1, y: 0}));
        let channel = match app.channels.channel_by_offset(
            app.session_view.eval_state.channel
        ) {
            Some(c) => c,
            None => &Channel::default(),
        };

        // make the app aware of the channel area so we can
        // scroll it correctly when its updated
        app.session_view.eval_state.eval_area.set(inner_area);

        frame.render_widget(
            Paragraph::new(
                channel.viewport(
                    inner_area.height,
                    app.session_view.eval_state.scroll.0
                ).join("\n").as_bytes().to_text().unwrap()
            ).scroll(
                (0, app.session_view.eval_state.scroll.1)
            ).style(app.theme().source_line),
            inner_area,
        );
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

    fn draw(app: &App, frame: &mut Frame, inner_area: Rect, area: Rect) {
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
    properties: Vec<&Property>,
    lines: &mut Vec<String>,
    level: usize,
) {
    for property in properties {
        let mut spans = vec![
            "  ".repeat(level),
            property.name.to_string(),
            if !property.name.is_empty() { " ".to_string() } else {"".to_string()},
            property.type_name(),
            " = ".to_string(),
            match &property.value {
                Some(s) => s.to_string(),
                None => "".to_string(),
            }
        ];

        let delimiters = match property.property_type {
            PropertyType::Array => ("[", "]"),
            _ => ("{", "}"),
        };

        if !property.children.is_empty() {
            spans.push(delimiters.0.to_string());
        }

        lines.push(spans.join(""));

        if !property.children.is_empty() {
            draw_properties(property.children.defined_properties(), lines, level + 1);
            lines.push(format!("{}{}", "  ".repeat(level), delimiters.1));
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
    use crate::{dbgp::client::Properties, theme::Theme};
    use anyhow::Result;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_draw_properties_empty() -> Result<()> {
        let mut lines = vec![];
        draw_properties(
            Vec::new(),
            &mut lines,
            0,
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
        prop1.children = Properties::from_properties(vec![prop2]);
        prop1.name = "foo".to_string();

        draw_properties(
            vec![&prop1],
            &mut lines,
            0,
        );
        assert_eq!(
            vec![
                "foo string = {", 
                "  bar string = ", 
                "}",
            ],
            lines
                .iter()
                .map(|l| { l.to_string() })
                .collect::<Vec<String>>()
        );
        Ok(())
    }

}
