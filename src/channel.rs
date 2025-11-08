use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

pub struct Channel {
    pub buffer: Arc<Mutex<Vec<String>>>,
    pub chunks: Vec<String>,
}

pub struct Channels {
    channels: HashMap<String,Channel>,
    channel_by_offset: Vec<String>,
}

impl Channels {
    pub fn names(&self) -> Vec<&str>
    {
        self.channels.keys().map(|t|t.as_str()).collect()
    }

    pub fn get(&self, name: &str) -> Option<&Channel> {
        self.channels.get(name)
    }

    pub fn get_mut(&mut self, name: &str) -> &Channel {
        self.channels.entry(name.to_string()).or_insert_with(|| {
            self.channel_by_offset.push(name.to_string());
            Channel::new()
        })
    }

    pub async fn unload(&mut self) {
        for entry in self.channels.iter_mut() {
            entry.1.unload().await
        }
    }

    pub fn new() -> Self {
        Self{
            channels: HashMap::new(),
            channel_by_offset: Vec::new(),
        }
    }

    pub(crate) fn count(&self) -> usize {
        self.channels.keys().len()
    }

    pub(crate) fn channel_by_offset(&self, channel: usize) -> Option<&Channel> {
        match self.channel_by_offset.get(channel) {
            Some(name) => self.channels.get(name),
            None => None,
        }
    }
}


impl Channel {
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

impl Default for Channel {
    fn default() -> Self {
        Self::new()
    }
}
