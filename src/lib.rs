use uuid::Uuid;
pub mod mindnode;
pub mod mmap;
pub mod opml;
pub mod smmx;
pub mod storage;
pub mod xmind;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub content: String,
    pub children: Vec<String>, // Node IDs
    pub parent: Option<String>,
    pub x: f32,
    pub y: f32,
    pub created: u64,
    pub modified: u64,
    #[serde(default)]
    pub icons: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MindMap {
    pub nodes: std::collections::HashMap<String, Node>,
    pub root_id: String,
    pub selected_node_id: String,
}

impl MindMap {
    pub fn new() -> Self {
        let root_id = Uuid::new_v4().to_string();
        let root = Node {
            id: root_id.clone(),
            content: "Central Node".to_string(),
            children: Vec::new(),
            parent: None,
            x: 0.0,
            y: 0.0,
            created: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            modified: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            icons: Vec::new(),
        };
        let mut nodes = std::collections::HashMap::new();
        nodes.insert(root_id.clone(), root);
        Self {
            nodes,
            root_id: root_id.clone(),
            selected_node_id: root_id,
        }
    }
}
