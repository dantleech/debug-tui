use tokio::sync::mpsc::Sender;

use crate::{dbgp::client::{DbgpClient, Init, Message}, event::input::AppEvent};

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
        match event {
            AppEvent::Run => match self.client.run().await? {
                Message::Response(r) => {
                    if r.status == "stopping" {
                        self.sender.send(AppEvent::Disconnect).await?;
                        return Ok(());
                    }
                    Ok(())
                },
                _ => Err(anyhow::anyhow!("Unexpected response")),
            },
            _ => Ok(()),
        }
    }

    pub(crate) fn disconnect(&mut self) {
        self.client.disonnect();
    }
}

