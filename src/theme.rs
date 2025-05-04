use ratatui::style::{Color, Style};

pub enum Theme {
    Dark,
}

impl Theme {
    pub fn scheme(&self) -> Scheme {
        match self {
            Theme::Dark => Scheme{
                syntax_variable: Style::default().fg(Color::LightBlue),
                syntax_type: Style::default().fg(Color::LightRed),
                syntax_type_object: Style::default().fg(Color::LightMagenta),
                syntax_literal: Style::default().fg(Color::LightBlue),
                syntax_literal_string: Style::default().fg(Color::LightGreen),
                syntax_label: Style::default().fg(Color::White),
                syntax_brace: Style::default().fg(Color::White),

                notification_info: Style::default().fg(Color::Black).bg(Color::Green),
                notification_error: Style::default().fg(Color::White).bg(Color::Red),

                pane_border_active: Style::default().fg(Color::Green),
                pane_border_inactive: Style::default().fg(Color::DarkGray),

                source_line: Style::default(),
                source_line_no: Style::default().fg(Color::Yellow),
                source_line_highlight: Style::default().bg(Color::Blue),
                source_annotation: Style::default().fg(Color::DarkGray),

                stack_line: Style::default().fg(Color::White),
            },
        }
    }
}

pub struct Scheme {
    pub syntax_variable: Style,
    pub syntax_type: Style,
    pub syntax_type_object: Style,
    pub syntax_literal: Style,
    pub syntax_literal_string: Style,
    pub syntax_label: Style,
    pub syntax_brace: Style,

    pub notification_info: Style,
    pub notification_error: Style,

    pub pane_border_active: Style,
    pub pane_border_inactive: Style,

    pub source_line: Style,
    pub source_line_no: Style,
    pub source_line_highlight: Style,
    pub source_annotation: Style,

    pub stack_line: Style,
}

pub enum Role {
}


