use std::{collections::{hash_map::Entry, HashMap}, sync::Arc};
use tokio::sync::Mutex;

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
        self.channel_by_offset.iter().map(|s|s.as_str()).collect()
    }

    pub fn get(&self, name: &str) -> Option<&Channel> {
        self.channels.get(name)
    }

    pub fn get_mut(&mut self, name: &str) -> &Channel {
        self.channels.entry(name.to_string()).or_insert_with(|| {
            self.channel_by_offset.push(name.to_string());
            Channel::new(name.to_string())
        })
    }

    pub async fn unload(&mut self, savepoint: usize) {
        for channel in self.channels.iter_mut() {
            channel.1.unload(savepoint).await
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

    pub(crate) async fn savepoint(&mut self, savepoint: usize) {
        for channel in self.channels.values_mut() {
            channel.savepoint(savepoint).await;
        }
    }

    pub(crate) fn reset(&mut self) {
        self.channels = HashMap::new();
        self.channel_by_offset = Vec::new();
    }
}

pub struct Channel {
    pub name: String,
    pub buffer: Arc<Mutex<String>>,
    pub lines: Vec<String>,
    pub savepoints: HashMap<usize,usize>,
}

impl Channel {
    pub async fn savepoint(&mut self, savepoint: usize) {
        self.savepoints.insert(savepoint, self.buffer.lock().await.len());
    }

    // unload the current buffer into a variable that is readable without a mutex lock.
    // TODO: this happens on each tick and could be more performant
    pub async fn unload(&mut self, savepoint: usize) {
        let content  = self.buffer.lock().await.clone();

        self.lines = match self.savepoints.entry(savepoint) {
            // if there is a savepoint return the content buffer up until that
            // savepoint
            Entry::Occupied(occupied_entry) => {
                let offset = occupied_entry.get();
                content[0..*offset].lines().map(|s|s.to_string()).collect()
            }
            Entry::Vacant(_) => {
                vec![]
            }
        };
    }

    pub fn new(name: String) -> Self {
        Self {
            name,
            buffer: Arc::new(Mutex::new(String::new())),
            lines: vec![],
            savepoints: HashMap::new(),
        }
    }

    pub(crate) async fn write(&self, join: String) {
        self.buffer.lock().await.push_str(join.as_str());
    }

    pub(crate) async fn writeln(&self, join: String) {
        self.write(join).await;
        self.buffer.lock().await.push('\n');
    }

    pub(crate) fn viewport(&self, height: u16, scroll: u16) -> &[String] {
        let y1 = scroll.min(self.lines.len() as u16);
        let y2 = (scroll + height).min(self.lines.len() as u16);

        &self.lines[
            (y1 as usize)..(y2 as usize)
        ]
    }
}

impl Default for Channel {
    fn default() -> Self {
        Self::new("default".to_string())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    pub async fn test_channel_lines() {
        let mut channel = Channel::new("test".to_string());
        channel.write("foobar".to_string()).await;
        channel.write("\nbarfoo\nbaz\none\ntwo".to_string()).await;
        channel.write("baf\nbaz\n".to_string()).await;
        
        assert_eq!(0, channel.lines.len());
        channel.savepoint(0).await;
        channel.unload(0).await;

        assert_eq!(6, channel.lines.len());
    }

    #[tokio::test]
    pub async fn test_savepoint() {
        let mut channel = Channel::new("test".to_string());
        channel.write("foobar\nbar".to_string()).await;
        channel.savepoint(0).await;
        channel.write("\nbarfoo\nbaz\none\ntwo".to_string()).await;
        channel.savepoint(1).await;
        channel.write("baf\nbaz\n".to_string()).await;
        
        assert_eq!(0, channel.lines.len());
        channel.savepoint(1).await;
        channel.unload(1).await;
        assert_eq!(7, channel.lines.len());
        channel.unload(0).await;
        assert_eq!(2, channel.lines.len());
    }

    #[tokio::test]
    pub async fn test_channel_lines_with_unterminated_previous() {
        let mut channel = Channel::default();
        channel.write("foobar".to_string()).await;
        channel.savepoint(0).await;
        channel.unload(0).await;
        assert_eq!(1, channel.lines.len());
        channel.write("barfoo".to_string()).await;
        channel.savepoint(0).await;
        channel.unload(0).await;
        assert_eq!(1, channel.lines.len());
        channel.write("barfoo\n".to_string()).await;
        channel.savepoint(0).await;
        channel.unload(0).await;
        assert_eq!(1, channel.lines.len());
    }

    #[tokio::test]
    pub async fn test_channel_lines_with_nothing() {
        let mut channel = Channel::default();
        channel.unload(100).await;
        
        assert_eq!(0, channel.lines.len());
        channel.write("".to_string()).await;
        channel.unload(100).await;
        assert_eq!(0, channel.lines.len());
    }

    #[test]
    pub fn test_viewport() {
        let mut channel = Channel::default();
        channel.lines.push("one".to_string());
        channel.lines.push("two".to_string());
        channel.lines.push("three".to_string());
        channel.lines.push("four".to_string());
        channel.lines.push("five".to_string());

        assert_eq!(vec![
            "one",
            "two",
        ], channel.viewport(2, 0));

        assert_eq!(vec![
            "two",
            "three",
        ], channel.viewport(2, 1));

        assert_eq!(vec![
            "five",
        ], channel.viewport(2, 4));

        assert_eq!(Vec::<String>::new(), channel.viewport(2, 5));
        assert_eq!(Vec::<String>::new(), channel.viewport(2, 6));

    }
}
