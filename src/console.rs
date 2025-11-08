use std::sync::Arc;
use tokio::sync::Mutex;

pub struct Console {
    pub buffer: Arc<Mutex<Vec<String>>>,
    pub chunks: Vec<String>,
}

impl Console {
    pub async fn unload(&mut self) {
        self.chunks.append(&mut self.buffer.lock().await.drain(0..).collect());
    }

    pub fn new() -> Self {
        Self {
            buffer: Arc::new(Mutex::new(vec![])),
            chunks: vec![]
        }
    }
}

impl Default for Console {
    fn default() -> Self {
        Self::new()
    }
}
