use super::View;
use crate::dbgp::client::ContinuationResponse;
use crate::dbgp::client::DbgpClient;
use crate::dbgp::client::Init;
use crate::event::input::AppEvent;
use crate::event::input::ServerStatus;
use anyhow::Result;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;
use std::str::FromStr;
use tokio::sync::mpsc::Sender;
use xmlem::Document;

pub struct SessionView {
    sender: Sender<AppEvent>,
}

impl View for SessionView {
    fn handle(&mut self, _app: &mut crate::app::App, _event: AppEvent) -> Option<AppEvent> {
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
        //    AppEvent::StepInto => {
        //        let response = client.step_into().await?;
        //        self.handle_continuation_response(response).await
        //    }
        //    AppEvent::StepOver => {
        //        let response = client.step_over().await?;
        //        self.handle_continuation_response(response).await
        //    }
        //    AppEvent::Run => {
        //        let response = client.run().await?;
        //        self.handle_continuation_response(response).await
        //    }
        //    _ => Ok(()),
        //}
        None
    }

    fn draw(
        &mut self,
        app: &mut crate::app::App,
        f: &mut ratatui::prelude::Buffer,
        area: ratatui::prelude::Rect,
    ) {
        match &app.source {
            Some(source_context) => {
                Paragraph::new(Line::from(vec![Span::styled(
                    source_context.filename.clone(),
                    Style::default().fg(Color::Green),
                )]))
                .render(area, f);
            }
            None => (),
        };
    }
}

impl SessionView {
    pub(crate) fn new(sender: Sender<AppEvent>) -> Self {
        Self { sender }
    }

    pub(crate) async fn handle(&mut self, event: AppEvent) -> Result<()> {}

    async fn handle_continuation_response(&mut self, r: ContinuationResponse) -> Result<()> {
        match r.status.as_str() {
            "stopping" => {
                self.sender
                    .send(AppEvent::UpdateStatus(ServerStatus::Stopping))
                    .await?;
            }
            "break" => {
                self.sender
                    .send(AppEvent::UpdateStatus(ServerStatus::Break))
                    .await?;
            }
            _ => {
                self.sender
                    .send(AppEvent::UpdateStatus(ServerStatus::Unknown(r.status)))
                    .await?;
            }
        }
        // update the source code
        let stack = app.client.get_stack().await?;
        match stack {
            Some(stack) => {
                self.sender
                    .send(AppEvent::RefreshSource(stack.filename, stack.line))
                    .await?;
            }
            None => (),
        };
        Ok(())
    }
}
