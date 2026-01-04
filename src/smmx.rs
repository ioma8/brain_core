use crate::{MindMap, Node};
use quick_xml::de::from_str;
use quick_xml::se::to_string;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

// SimpleMind XML Structure (Simplified)

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename = "simplemind-mindmaps")]
pub struct SmmxRoot {
    #[serde(rename = "mindmap")]
    pub mindmap: SmmxMindMap,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct SmmxMindMap {
    #[serde(rename = "topics")]
    pub topics: SmmxTopics,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct SmmxTopics {
    #[serde(rename = "topic")]
    pub topic: Vec<SmmxTopic>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct SmmxTopic {
    #[serde(rename = "@id")]
    pub id: String, // SimpleMind uses integer IDs usually, but string is safer for generic
    #[serde(rename = "@text")]
    pub text: String,
    #[serde(rename = "children", default, skip_serializing_if = "Option::is_none")]
    pub children: Option<SmmxChildren>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct SmmxChildren {
    #[serde(rename = "topics")]
    pub topics: SmmxTopics,
}

pub fn to_smmx(map: &MindMap) -> Result<String, String> {
    let root_node = map.nodes.get(&map.root_id).ok_or("Root node not found")?;

    // SimpleMind IDs are usually integers. We might need to map UUIDs to integers if strict.
    // But let's try using UUIDs as strings first.

    let smmx_root_topic = node_to_smmx_topic(root_node, map);

    let smmx_root = SmmxRoot {
        mindmap: SmmxMindMap {
            topics: SmmxTopics {
                topic: vec![smmx_root_topic],
            },
        },
    };

    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str(&to_string(&smmx_root).map_err(|e| e.to_string())?);
    Ok(xml)
}

fn node_to_smmx_topic(node: &Node, map: &MindMap) -> SmmxTopic {
    let mut children_vec = Vec::new();
    for child_id in &node.children {
        if let Some(child) = map.nodes.get(child_id) {
            children_vec.push(node_to_smmx_topic(child, map));
        }
    }

    let children = if children_vec.is_empty() {
        None
    } else {
        Some(SmmxChildren {
            topics: SmmxTopics {
                topic: children_vec,
            },
        })
    };

    SmmxTopic {
        id: node.id.clone(),
        text: node.content.clone(),
        children,
    }
}

pub fn from_smmx(xml: &str) -> Result<MindMap, String> {
    let smmx_root: SmmxRoot = from_str(xml).map_err(|e| e.to_string())?;

    let mut nodes = HashMap::new();

    if smmx_root.mindmap.topics.topic.is_empty() {
        return Ok(MindMap::new());
    }

    let root_id = smmx_topic_to_node(&smmx_root.mindmap.topics.topic[0], None, &mut nodes);

    Ok(MindMap {
        nodes,
        root_id: root_id.clone(),
        selected_node_id: root_id,
    })
}

fn smmx_topic_to_node(
    topic: &SmmxTopic,
    parent_id: Option<&str>,
    nodes: &mut HashMap<String, Node>,
) -> String {
    let id = Uuid::new_v4().to_string(); // Generate new UUIDs to avoid ID conflicts or format issues

    let mut children_ids = Vec::new();
    if let Some(children) = &topic.children {
        for child in &children.topics.topic {
            children_ids.push(smmx_topic_to_node(child, Some(&id), nodes));
        }
    }

    let node = Node {
        id: id.clone(),
        content: topic.text.clone(),
        children: children_ids,
        parent: parent_id.map(|s| s.to_string()),
        x: 0.0,
        y: 0.0,
        created: now_millis(),
        modified: now_millis(),
        icons: Vec::new(),
    };

    nodes.insert(id.clone(), node);
    id
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smmx_serialization() {
        let mut map = MindMap::new();
        let root_id = map.root_id.clone();
        map.nodes.get_mut(&root_id).unwrap().content = "Root Smmx".to_string();

        map.add_child(&root_id, "Child 1".to_string()).unwrap();

        let xml = to_smmx(&map).unwrap();
        println!("DEBUG XML: {}", xml);
        assert!(!xml.is_empty());

        let loaded_map = from_smmx(&xml).unwrap();
        let root = loaded_map.nodes.get(&loaded_map.root_id).unwrap();
        assert_eq!(root.content, "Root Smmx");
        assert_eq!(root.children.len(), 1);
    }
}
