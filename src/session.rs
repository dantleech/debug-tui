use crate::dbgp::client::ContinuationResponse;
use crate::dbgp::client::DbgpClient;
use crate::dbgp::client::Init;
use crate::event::input::AppEvent;
use crate::event::input::ServerStatus;
use anyhow::Result;
use std::str::FromStr;
use tokio::sync::mpsc::Sender;
use xmlem::Document;

pub struct Session {
    client: DbgpClient,
    sender: Sender<AppEvent>,
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

    pub(crate) async fn handle(&mut self, event: AppEvent) -> Result<()> {
        let client = &mut self.client;
        match event {
            AppEvent::ExecCommand(cmd) => {
                let response = self.client.exec_raw(cmd).await?;
                let doc = Document::from_str(response.as_str())?;
                self.sender
                    .send(AppEvent::ExecCommandResponse(doc.to_string_pretty()))
                    .await?;
                return Ok(());
            }
            AppEvent::RefreshSource(filename, line_no) => {
                let source = self.client.source(filename.clone()).await?;
                self.sender
                    .send(AppEvent::UpdateSourceContext(
                        source,
                        filename.clone(),
                        line_no,
                    ))
                    .await?;
                Ok(())
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
