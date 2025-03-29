use crate::{dbgp::client::{DbgpClient, Init}, event::input::AppEvent};

pub struct Session {
    client: DbgpClient,
}
impl Session {
    pub(crate) fn new(client: DbgpClient) -> Self {
        Self { client }
    }

    pub(crate) async fn init(&mut self) -> Result<Init, anyhow::Error> {
        match self.client.read().await? {
            crate::dbgp::client::Response::Init(i) => Ok(i),
        }
    }

    pub(crate) async fn handle(&mut self, event: AppEvent) -> Result<(), anyhow::Error> {
        match event {
            AppEvent::Run => {
                self.client.run().await?;
                return Ok(());
            },
            _ => Ok(()),
        }
    }
}

