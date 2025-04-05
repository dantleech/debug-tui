use super::View;
use crate::app::App;
use crate::app::AppState;
use crate::app::InputMode;
use crate::dbgp::client::ContinuationResponse;
use crate::event::input::AppEvent;
use crate::event::input::ServerStatus;
use anyhow::Result;
use crossterm::event::KeyCode;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;
use ratatui::Frame;
use tokio::sync::mpsc::Sender;

pub struct SessionView {
    sender: Sender<AppEvent>,
}

impl View for SessionView {
    fn handle(app: &App, event: AppEvent) -> Option<AppEvent> {
        match event {
            AppEvent::Input(e) => {
                if app.input_mode != InputMode::Command {
                    if let KeyCode::Char(char) = e.code {
                        return match char {
                            'r' => Some(AppEvent::Run),
                            'n' => Some(AppEvent::StepInto),
                            'o' => Some(AppEvent::StepOver),
                            _ => None,
                        }
                    }
                }
            }
            _ => ()
        }
        //match event {
        //    AppEvent::ExecCommand(cmd) => {
        //        tokio::spawn(async move {
        //            let response = app.client.exec_raw(cmd).await;
        //            let doc = Document::from_str(response.unwrap().as_str());
        //            self.sender.blocking_send(AppEvent::ExecCommandResponse(doc.unwrap().to_string_pretty()));
        //        });
        //        None
        //    }
        //    AppEvent::RefreshSource(filename, line_no) => {
        //        tokio::spawn(async move {
        //            let source = app.client.source(filename.clone()).await?;
        //            self.sender
        //                .send(AppEvent::UpdateSourceContext(
        //                    source,
        //                    filename.clone(),
        //                    line_no,
        //                ))
        //                .await?;
        //        });
        //    }
        //    _ => Ok(()),
        //}
        None
    }

    fn draw(
        app: &mut App,
        f: &mut Frame,
        area: ratatui::prelude::Rect,
    ) {
        match &app.source {
            Some(source_context) => {
                f.render_widget(Paragraph::new(Line::from(vec![Span::styled(
                    source_context.filename.clone(),
                    Style::default().fg(Color::Green),
                )])), area);
            }
            None => (),
        };
    }
}

impl SessionView {
    pub(crate) fn new(sender: Sender<AppEvent>) -> Self {
        Self { sender }
    }
}
