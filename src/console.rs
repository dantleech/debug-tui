use std::sync::Arc;

use ratatui::text::Line;
use tokio::sync::Mutex;

pub struct Console {
    pub buffer: Arc<Mutex<Vec<String>>>,
    pub lines: Vec<String>,
}

impl Console {
    pub async fn unload(&mut self) {
        self.lines.append(&mut self.buffer.lock().await.to_vec());
        self.buffer.lock().await.clear();
    }
    pub fn new() -> Self {
        Self { buffer: Arc::new(Mutex::new(vec![])), lines: vec![] }
    }
}
