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

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Navigation {
    Up,
    Down,
    Left,
    Right,
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

    pub fn add_child(&mut self, parent_id: &str, content: String) -> Result<String, String> {
        if !self.nodes.contains_key(parent_id) {
            return Err("Parent node not found".to_string());
        }

        let new_id = Uuid::new_v4().to_string();
        let new_node = Node {
            id: new_id.clone(),
            content,
            children: Vec::new(),
            parent: Some(parent_id.to_string()),
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

        self.nodes.insert(new_id.clone(), new_node);

        let parent = self.nodes.get_mut(parent_id).unwrap();
        parent.children.push(new_id.clone());

        Ok(new_id)
    }

    pub fn change_node(&mut self, node_id: &str, content: String) -> Result<(), String> {
        if let Some(node) = self.nodes.get_mut(node_id) {
            node.content = content;
            node.modified = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            Ok(())
        } else {
            Err("Node not found".to_string())
        }
    }

    pub fn remove_node(&mut self, node_id: &str) -> Result<(), String> {
        if node_id == self.root_id {
            return Err("Cannot remove root node".to_string());
        }

        let parent_id = self
            .nodes
            .get(node_id)
            .ok_or("Node not found")?
            .parent
            .clone()
            .ok_or("Node has no parent")?;

        if let Some(parent) = self.nodes.get_mut(&parent_id) {
            parent.children.retain(|id| id != node_id);
        }

        let mut to_remove = vec![node_id.to_string()];
        let mut i = 0;
        while i < to_remove.len() {
            let curr = &to_remove[i];
            if let Some(node) = self.nodes.get(curr) {
                to_remove.extend(node.children.clone());
            }
            i += 1;
        }

        for id in to_remove {
            self.nodes.remove(&id);
        }

        Ok(())
    }

    pub fn add_sibling(&mut self, node_id: &str, content: String) -> Result<String, String> {
        if node_id == self.root_id {
            return Err("Cannot add sibling to root".to_string());
        }

        let parent_id = self
            .nodes
            .get(node_id)
            .ok_or("Node not found")?
            .parent
            .clone()
            .ok_or("Node has no parent")?;

        let new_id = Uuid::new_v4().to_string();
        let new_node = Node {
            id: new_id.clone(),
            content,
            children: Vec::new(),
            parent: Some(parent_id.clone()),
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

        self.nodes.insert(new_id.clone(), new_node);

        if let Some(parent) = self.nodes.get_mut(&parent_id) {
            if let Some(pos) = parent.children.iter().position(|id| id == node_id) {
                parent.children.insert(pos + 1, new_id.clone());
            } else {
                parent.children.push(new_id.clone());
            }
        }

        Ok(new_id)
    }

    pub fn add_icon(&mut self, node_id: &str, icon: String) -> Result<(), String> {
        if let Some(node) = self.nodes.get_mut(node_id) {
            node.icons.push(icon);
            node.modified = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            Ok(())
        } else {
            Err("Node not found".to_string())
        }
    }

    pub fn remove_last_icon(&mut self, node_id: &str) -> Result<(), String> {
        if let Some(node) = self.nodes.get_mut(node_id) {
            node.icons.pop();
            node.modified = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            Ok(())
        } else {
            Err("Node not found".to_string())
        }
    }

    pub fn compute_layout(&mut self) {
        let root_id = self.root_id.clone();
        self.layout_node(&root_id, 0.0, 0.0);
    }

    fn layout_node(&mut self, node_id: &str, x: f32, start_y: f32) -> f32 {
        let (children, node_width) = if let Some(node) = self.nodes.get(node_id) {
            // Estimate node width: ~8 pixels per character + 20 padding + 20 per icon
            let text_width = node.content.len() as f32 * 8.0;
            let icons_width = node.icons.len() as f32 * 20.0;
            let width = text_width + icons_width + 20.0;
            (node.children.clone(), width.max(100.0)) // minimum 100px
        } else {
            return 0.0;
        };

        let node_h = 50.0;
        let gap = 50.0; // gap between parent right edge and child left edge

        if children.is_empty() {
            if let Some(node) = self.nodes.get_mut(node_id) {
                node.x = x;
                node.y = start_y;
            }
            return node_h;
        }

        // Child x position is parent x + parent width + gap
        let child_x = x + node_width + gap;

        let mut current_y = start_y;
        for child_id in children {
            let h = self.layout_node(&child_id, child_x, current_y);
            current_y += h;
        }

        let total_h = current_y - start_y;
        let parent_y = start_y + (total_h / 2.0);

        if let Some(node) = self.nodes.get_mut(node_id) {
            node.x = x;
            node.y = parent_y;
        }

        total_h
    }

    pub fn select_node(&mut self, node_id: &str) -> Result<(), String> {
        if self.nodes.contains_key(node_id) {
            self.selected_node_id = node_id.to_string();
            Ok(())
        } else {
            Err("Node not found".to_string())
        }
    }

    pub fn navigate(&mut self, direction: Navigation) {
        let current_id = self.selected_node_id.clone();

        let new_selection = match direction {
            Navigation::Right => {
                if let Some(node) = self.nodes.get(&current_id) {
                    node.children.first().cloned()
                } else {
                    None
                }
            }
            Navigation::Left => {
                if let Some(node) = self.nodes.get(&current_id) {
                    node.parent.clone()
                } else {
                    None
                }
            }
            Navigation::Down => {
                if let Some(node) = self.nodes.get(&current_id) {
                    if let Some(parent_id) = &node.parent {
                        if let Some(parent) = self.nodes.get(parent_id) {
                            if let Some(pos) =
                                parent.children.iter().position(|id| id == &current_id)
                            {
                                if pos + 1 < parent.children.len() {
                                    Some(parent.children[pos + 1].clone())
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            Navigation::Up => {
                if let Some(node) = self.nodes.get(&current_id) {
                    if let Some(parent_id) = &node.parent {
                        if let Some(parent) = self.nodes.get(parent_id) {
                            if let Some(pos) =
                                parent.children.iter().position(|id| id == &current_id)
                            {
                                if pos > 0 {
                                    Some(parent.children[pos - 1].clone())
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        };

        if let Some(id) = new_selection {
            self.selected_node_id = id;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mindmap_initialization() {
        let map = MindMap::new();
        assert!(map.nodes.contains_key(&map.root_id));
        let root = map.nodes.get(&map.root_id).unwrap();
        assert_eq!(root.content, "Central Node");
        assert!(root.children.is_empty());
        assert!(root.parent.is_none());
    }

    #[test]
    fn test_add_child() {
        let mut map = MindMap::new();
        let root_id = map.root_id.clone();

        let child_id = map
            .add_child(&root_id, "Child 1".to_string())
            .expect("Failed to add child");

        let child = map.nodes.get(&child_id).unwrap();
        assert_eq!(child.content, "Child 1");
        assert_eq!(child.parent, Some(root_id.clone()));

        let root = map.nodes.get(&root_id).unwrap();
        assert!(root.children.contains(&child_id));
    }

    #[test]
    fn test_change_node() {
        let mut map = MindMap::new();
        let root_id = map.root_id.clone();

        map.change_node(&root_id, "New Root Content".to_string())
            .expect("Failed to change node");

        let root = map.nodes.get(&root_id).unwrap();
        assert_eq!(root.content, "New Root Content");
    }

    #[test]
    fn test_remove_node() {
        let mut map = MindMap::new();
        let root_id = map.root_id.clone();

        let child_id = map.add_child(&root_id, "Child".to_string()).unwrap();
        let grandchild_id = map.add_child(&child_id, "GrandChild".to_string()).unwrap();

        // Ensure structure exists
        assert!(map.nodes.contains_key(&child_id));
        assert!(map.nodes.contains_key(&grandchild_id));

        map.remove_node(&child_id).expect("Failed to remove node");

        // Ensure removed recursively
        assert!(!map.nodes.contains_key(&child_id));
        assert!(!map.nodes.contains_key(&grandchild_id));

        // Ensure parent link removed
        let root = map.nodes.get(&root_id).unwrap();
        assert!(!root.children.contains(&child_id));
    }

    #[test]
    fn test_add_sibling() {
        let mut map = MindMap::new();
        let root_id = map.root_id.clone();

        let child1_id = map.add_child(&root_id, "Child 1".to_string()).unwrap();

        // Add sibling to Child 1
        let sibling_id = map
            .add_sibling(&child1_id, "Sibling 1".to_string())
            .expect("Failed to add sibling");

        let root = map.nodes.get(&root_id).unwrap();
        assert_eq!(root.children.len(), 2);
        assert_eq!(root.children[0], child1_id);
        assert_eq!(root.children[1], sibling_id);

        let sibling = map.nodes.get(&sibling_id).unwrap();
        assert_eq!(sibling.parent, Some(root_id.clone()));

        // Test adding sibling to Root (Should fail)
        assert!(
            map.add_sibling(&root_id, "Root Sibling".to_string())
                .is_err()
        );
    }

    #[test]
    fn test_layout() {
        let mut map = MindMap::new();
        let root_id = map.root_id.clone();

        let child1 = map.add_child(&root_id, "Child 1".to_string()).unwrap();
        let child2 = map.add_child(&root_id, "Child 2".to_string()).unwrap();
        let grand1 = map.add_child(&child1, "Grand 1".to_string()).unwrap();

        map.compute_layout();

        let root = map.nodes.get(&root_id).unwrap();
        let c1 = map.nodes.get(&child1).unwrap();
        let c2 = map.nodes.get(&child2).unwrap();
        let g1 = map.nodes.get(&grand1).unwrap();

        // Check X positions (Rightwards growth)
        assert!(c1.x > root.x);
        assert!(c2.x > root.x);
        assert!(g1.x > c1.x);

        // Check Y separation
        assert!(c1.y != c2.y);
    }

    #[test]
    fn test_navigate() {
        let mut map = MindMap::new();
        let root_id = map.root_id.clone();

        let child1 = map.add_child(&root_id, "C1".to_string()).unwrap();
        let child2 = map.add_child(&root_id, "C2".to_string()).unwrap();

        // Default selection is root
        assert_eq!(map.selected_node_id, root_id);

        // Navigate Right -> First child
        map.navigate(Navigation::Right);
        assert_eq!(map.selected_node_id, child1);

        // Navigate Down -> Next sibling
        map.navigate(Navigation::Down);
        assert_eq!(map.selected_node_id, child2);

        // Navigate Up -> Prev sibling
        map.navigate(Navigation::Up);
        assert_eq!(map.selected_node_id, child1);

        // Navigate Left -> Parent
        map.navigate(Navigation::Left);
        assert_eq!(map.selected_node_id, root_id);
    }
}
