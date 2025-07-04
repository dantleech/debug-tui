
use ratatui::style::Color;
use ratatui::style::Style;

#[derive(Debug)]
pub enum Theme {
    Dark,
    SolarizedDark,
    SolarizedLight,
}

impl Theme {
    pub fn next(&self) -> Theme {
        match self {
            Theme::Dark => Self::SolarizedDark,
            Theme::SolarizedDark => Self::SolarizedLight,
            Theme::SolarizedLight => Self::Dark,
        }
    }
    pub fn scheme(&self) -> Scheme {
        match self {
            Theme::SolarizedLight => {
                let mut scheme = Self::SolarizedDark.scheme();
                scheme.background = Style::default().bg(Solarized::Base3.to_color());

                scheme.widget_active = Style::default().fg(Solarized::Green.to_color()).bg(Solarized::Base2.to_color());
                scheme.widget_inactive = Style::default().fg(Solarized::Base03.to_color()).bg(Solarized::Base3.to_color());

                scheme.pane_border_active = Style::default().fg(Solarized::Base01.to_color()).bg(Solarized::Base3.to_color());

                scheme.pane_border_inactive = Style::default().fg(Solarized::Base02.to_color()).bg(Solarized::Base3.to_color());

                scheme.source_line = scheme.source_line.fg(Solarized::Base00.to_color());
                scheme.source_line_highlight = scheme.source_line_highlight.bg(Solarized::Base2.to_color()).fg(Solarized::Base01.to_color());

                scheme.widget_mode_debug = scheme.widget_mode_debug.fg(Solarized::Base03.to_color()).bg(Solarized::Base3.to_color());
                scheme.widget_mode_history = scheme.widget_mode_debug.bg(Solarized::Base3.to_color()).fg(Solarized::Red.to_color());

                scheme
            }
            Theme::SolarizedDark => Scheme{
                syntax_variable: Style::default().fg(Solarized::Base00.to_color()),
                syntax_type: Style::default().fg(Solarized::Cyan.to_color()),
                syntax_type_object: Style::default().fg(Solarized::Orange.to_color()),
                syntax_literal: Style::default().fg(Solarized::Blue.to_color()),
                syntax_literal_string: Style::default().fg(Solarized::Green.to_color()),
                syntax_label: Style::default().fg(Solarized::Base00.to_color()),
                syntax_brace: Style::default().fg(Solarized::Base01.to_color()),
                notification_info: Style::default().fg(Solarized::Green.to_color()),
                notification_error: Style::default().fg(Solarized::Red.to_color()),
                notification_warning: Style::default().fg(Solarized::Yellow.to_color()),
                pane_border_active: Style::default().fg(Solarized::Base01.to_color()),
                pane_border_inactive: Style::default().fg(Solarized::Base02.to_color()),
                source_line: Style::default().fg(Solarized::Base1.to_color()),
                source_line_no: Style::default().fg(Solarized::Yellow.to_color()),
                source_line_highlight: Style::default().bg(Solarized::Base02.to_color()).fg(Solarized::Base3.to_color()),
                source_annotation: Style::default().fg(Solarized::Magenta.to_color()),
                stack_line: Style::default().fg(Solarized::Base1.to_color()),

                widget_active: Style::default().fg(Solarized::Base02.to_color()).bg(Solarized::Green.to_color()),
                widget_inactive: Style::default().fg(Solarized::Base1.to_color()).bg(Solarized::Base03.to_color()),
                widget_mode_debug: Style::default().fg(Solarized::Base1.to_color()).bg(Solarized::Base03.to_color()),
                widget_mode_history: Style::default().fg(Solarized::Red.to_color()).bg(Solarized::Base03.to_color()),

                background: Style::default().bg(Color::Black),
                cursor: Style::default().bg(Color::White),
            },
            Theme::Dark => Scheme {
                syntax_variable: Style::default().fg(Color::LightBlue),
                syntax_type: Style::default().fg(Color::LightRed),
                syntax_type_object: Style::default().fg(Color::LightMagenta),
                syntax_literal: Style::default().fg(Color::LightBlue),
                syntax_literal_string: Style::default().fg(Color::LightGreen),
                syntax_label: Style::default().fg(Color::White),
                syntax_brace: Style::default().fg(Color::White),

                notification_info: Style::default().fg(Color::Black).bg(Color::Green),
                notification_error: Style::default().fg(Color::Yellow),
                notification_warning: Style::default().fg(Color::White).bg(Color::Yellow),

                pane_border_active: Style::default().fg(Color::Green),
                pane_border_inactive: Style::default().fg(Color::DarkGray),

                source_line: Style::default().fg(Color::White),
                source_line_no: Style::default().fg(Color::Yellow),
                source_line_highlight: Style::default().bg(Color::Blue),
                source_annotation: Style::default().fg(Color::DarkGray),

                stack_line: Style::default().fg(Color::White),

                widget_active: Style::default().fg(Color::Black).bg(Color::Green),
                widget_inactive: Style::default().fg(Color::Black).bg(Color::Yellow),
                widget_mode_debug: Style::default().bg(Color::Blue),
                widget_mode_history: Style::default().bg(Color::Red),
                background: Style::default().bg(Color::Black),
                cursor: Style::default().bg(Color::White)
            },
        }
    }
}

pub struct Scheme {
    pub background: Style,
    pub syntax_variable: Style,
    pub syntax_type: Style,
    pub syntax_type_object: Style,
    pub syntax_literal: Style,
    pub syntax_literal_string: Style,
    pub syntax_label: Style,
    pub syntax_brace: Style,

    pub notification_info: Style,
    pub notification_error: Style,
    pub notification_warning: Style,

    pub pane_border_active: Style,
    pub pane_border_inactive: Style,

    pub source_line: Style,
    pub source_line_no: Style,
    pub source_line_highlight: Style,
    pub source_annotation: Style,

    pub stack_line: Style,

    pub widget_active: Style,
    pub widget_inactive: Style,
    pub widget_mode_debug: Style,
    pub widget_mode_history: Style,
    pub cursor: Style,
}

pub enum Role {}

pub enum Solarized {
    Base03,
    Base02,
    Base01,
    Base00,
    Base0,
    Base1,
    Base2,
    Base3,
    Yellow,
    Orange,
    Red,
    Magenta,
    Violet,
    Blue,
    Cyan,
    Green,
}

impl Solarized {
    fn to_color(&self) -> Color {
        match self {
            Solarized::Base03 => Color::Rgb(0, 43, 54),
            Solarized::Base02 => Color::Rgb(7, 54, 66),
            Solarized::Base01 => Color::Rgb(88, 110, 117),
            Solarized::Base00 => Color::Rgb(101, 123, 131),
            Solarized::Base0 => Color::Rgb(131, 148, 150),
            Solarized::Base1 => Color::Rgb(147, 161, 161),
            Solarized::Base2 => Color::Rgb(238, 232, 213),
            Solarized::Base3 => Color::Rgb(253, 246, 227),
            Solarized::Yellow => Color::Rgb(181, 137, 0),
            Solarized::Orange => Color::Rgb(203, 75, 22),
            Solarized::Red => Color::Rgb(220, 50, 47),
            Solarized::Magenta => Color::Rgb(211, 54, 130),
            Solarized::Violet => Color::Rgb(108, 113, 196),
            Solarized::Blue => Color::Rgb(38, 139, 210),
            Solarized::Cyan => Color::Rgb(42, 161, 152),
            Solarized::Green => Color::Rgb(133, 153, 0),
        }
    }
}
