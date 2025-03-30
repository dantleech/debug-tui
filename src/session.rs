use tokio::sync::mpsc::Sender;

use crate::{dbgp::client::{ContinuationResponse, DbgpClient, Init}, event::input::{AppEvent, ServerStatus}};

pub struct Session {
    client: DbgpClient,
    sender: Sender<AppEvent>,
}

impl Session {
    pub(crate) fn new(client: DbgpClient, sender: Sender<AppEvent>) -> Self {
        Self { client, sender }
    }

    pub(crate) async fn init(&mut self) -> Result<Init, anyhow::Error> {
        match self.client.read().await? {
            crate::dbgp::client::Message::Init(i) => Ok(i),
            _ => Err(anyhow::anyhow!("Unexpected response")),
        }
    }

    pub(crate) async fn handle(&mut self, event: AppEvent) -> Result<(), anyhow::Error> {
        let client = &mut self.client;
        match event {
            AppEvent::ExecCommand(cmd) => {
                let response = self.client.exec_raw(cmd).await;
                self.sender.send(AppEvent::ExecCommandResponse(response?)).await?;
                Ok(())
            },
            AppEvent::RefreshSource(filename, line_no) => {
                let source = self.client.source(filename.clone()).await?;
                self.sender.send(AppEvent::UpdateSourceContext(source, filename.clone(), line_no)).await?;
                Ok(())
            },
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

    async fn handle_continuation_response(&mut self, r: ContinuationResponse) -> Result<(), anyhow::Error> {
        match r.status.as_str() {
            "stopping" => {
                self.sender.send(AppEvent::UpdateStatus(ServerStatus::Stopping)).await?;
            },
            "break" => {
                self.sender.send(AppEvent::UpdateStatus(ServerStatus::Break)).await?;
            },
            _ => {
                self.sender.send(AppEvent::UpdateStatus(ServerStatus::Unknown(r.status))).await?;
            }
        }
        // update the source code
        let stack = self.client.get_stack().await?;
        match stack {
            Some(stack) => {
                self.sender.send(AppEvent::RefreshSource(stack.filename,stack.line)).await?;
            },
            None => (),
        };
        Ok(())
    }
}

