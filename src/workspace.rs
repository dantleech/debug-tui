use crate::dbgp::client::DbgpClient;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct Document {
    pub filename: String,
    pub text: String,
}

pub struct Workspace {
    client: Arc<Mutex<DbgpClient>>,
    documents: HashMap<String, Document>,
}

impl Workspace {
    pub fn new(client: Arc<Mutex<DbgpClient>>) -> Self {
        Workspace {
            client,
            documents: HashMap::new(),
        }
    }

    pub async fn open(&mut self, filename: String) -> &Document {
        let client = Arc::clone(&self.client);

        let entry = self.documents.entry(filename.clone());
        if let Entry::Vacant(entry) = entry {
            let source = client
                .lock()
                .await
                .source(filename.to_string())
                .await
                .expect("Could not retrieve source");
            entry.insert(Document {
                filename: "".to_string(),
                text: source.clone(),
            });
        };

        self.documents.get(filename.as_str()).unwrap()
    }
}
