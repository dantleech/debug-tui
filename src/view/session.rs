use anyhow::Result;
use std::str::FromStr;

use tokio::sync::mpsc::Sender;
use xmlem::Document;

use crate::{
    dbgp::client::{ContinuationResponse, DbgpClient, Init},
    event::input::{AppEvent, ServerStatus},
};

use super::View;

pub struct Session {
    client: DbgpClient,
    sender: Sender<AppEvent>,
}

impl View for Session {
    fn handle(&mut self, app: &mut crate::app::App, event: AppEvent) -> Option<AppEvent> {
        let client = &mut self.client;
        match event {
            AppEvent::ExecCommand(cmd) => {
                tokio::spawn(async move {
                    let response = self.client.exec_raw(cmd).await;
                    let doc = Document::from_str(response.unwrap().as_str());
                    self.sender.blocking_send(AppEvent::ExecCommandResponse(doc.unwrap().to_string_pretty()));
                });
                None
            }
            AppEvent::RefreshSource(filename, line_no) => {
                tokio::spawn(async move {
                    let source = self.client.source(filename.clone()).await?;
                    self.sender
                        .send(AppEvent::UpdateSourceContext(
                            source,
                            filename.clone(),
                            line_no,
                        ))
                        .await?;
                });
            }
            AppEvent::StepInto => {
                let response = client.step_into().await?;
                self.handle_continuation_response(response).await
            }
            AppEvent::StepOver => {
                let response = client.step_over().await?;
                self.handle_continuation_response(response).await
            }
            AppEvent::Run => {
                let response = client.run().await?;
                self.handle_continuation_response(response).await
            }
            _ => Ok(()),
        }
    }

    fn draw(
        &mut self,
        app: &mut crate::app::App,
        f: &mut ratatui::prelude::Buffer,
        area: ratatui::prelude::Rect,
    ) {
        todo!()
    }
}

impl Session {
    pub(crate) fn new(client: DbgpClient, sender: Sender<AppEvent>) -> Self {
        Self { client, sender }
    }

    pub(crate) async fn init(&mut self) -> Result<Init> {
        match self.client.read_and_parse().await? {
            crate::dbgp::client::Message::Init(i) => Ok(i),
            _ => anyhow::bail!("Unexpected response"),
        }
    }

    pub(crate) async fn handle(&mut self, event: AppEvent) -> Result<()> {}

    pub(crate) async fn disconnect(&mut self) {
        self.client.disonnect().await
    }

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
        let stack = self.client.get_stack().await?;
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
