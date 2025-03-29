use tokio::sync::mpsc::Sender;

use crate::{dbgp::client::{DbgpClient, Init, Message, Response}, event::input::{AppEvent, ServerStatus}};

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
            AppEvent::StepInto => {
                let response = client.step_into().await?;
                self.handle_response(response).await
            }
            AppEvent::Run => {
                let response = client.run().await?;
               self.handle_response(response).await
            }
            _ => Ok(()),
        }
    }

    pub(crate) fn disconnect(&mut self) {
        self.client.disonnect();
    }

    async fn handle_response(&mut self, r: Response) -> Result<(), anyhow::Error> {
        if r.status == "stopping" {
            self.sender.send(AppEvent::UpdateStatus(ServerStatus::Stopping)).await?;
        }
        if r.status == "break" {
            self.sender.send(AppEvent::UpdateStatus(ServerStatus::Break)).await?;
        }
        self.sender.send(AppEvent::UpdateStatus(ServerStatus::Unknown(r.status))).await?;
        Ok(())
    }
}

