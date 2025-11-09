use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

pub struct Channel {
    pub buffer: Arc<Mutex<Vec<String>>>,
    pub lines: Vec<String>,
}

pub struct Channels {
    channels: HashMap<String,Channel>,
    channel_by_offset: Vec<String>,
}

impl Default for Channels {
    fn default() -> Self {
        Self::new()
    }
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

    pub(crate) fn offset_by_name(&self, name: String) -> Option<usize> {
        self.channel_by_offset.iter().position(|n|*n==name)
    }
}


impl Channel {
    pub async fn unload(&mut self) {
        let chunks: Vec<String> = self.buffer.lock().await.drain(0..).collect();
        let content = chunks.join("");
        let mut lines: Vec<String> = content.lines().map(|s|s.to_string()).collect();

        // content.lines() will ignore trailing new lines. we explicitly
        // add a new line if the last character was a new line.
        if let Some(char) = content.chars().last() {
            if char == '\n' {
                lines.push("".to_string());
            }
        }

        if lines.is_empty() {
            return;
        }
        if let Some(l) = &mut self.lines.last_mut() {
            let first = lines.first().unwrap();
            l.push_str(first.as_str());
            self.lines.append(&mut lines[1..].to_vec());
            return;
        }
        self.lines.append(&mut lines);
    }

    pub fn new() -> Self {
        Self {
            buffer: Arc::new(Mutex::new(vec![])),
            lines: vec![]
        }
    }

    pub(crate) async fn write(&self, join: String) {
        self.buffer.lock().await.push(join);
    }

    pub(crate) async fn writeln(&self, join: String) {
        self.write(join).await;
        self.buffer.lock().await.push("\n".to_string());
    }
}

impl Default for Channel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    pub async fn test_channel_lines() {
        let mut channel = Channel::new();
        channel.write("foobar".to_string()).await;
        channel.write("\nbarfoo\nbaz\none\ntwo".to_string()).await;
        channel.write("baf\nbaz\n".to_string()).await;
        
        assert_eq!(0, channel.lines.len());
        channel.unload().await;

        assert_eq!(7, channel.lines.len());
    }

    #[tokio::test]
    pub async fn test_channel_lines_with_unterminated_previous() {
        let mut channel = Channel::new();
        channel.write("foobar".to_string()).await;
        channel.unload().await;
        assert_eq!(1, channel.lines.len());
        channel.write("barfoo".to_string()).await;
        channel.unload().await;
        assert_eq!(1, channel.lines.len());
        channel.write("barfoo\n".to_string()).await;
        channel.unload().await;
        assert_eq!(2, channel.lines.len());
    }

    #[tokio::test]
    pub async fn test_channel_lines_with_nothing() {
        let mut channel = Channel::new();
        channel.unload().await;
        
        assert_eq!(0, channel.lines.len());
        channel.write("".to_string()).await;
        channel.unload().await;
        assert_eq!(0, channel.lines.len());
    }
}
