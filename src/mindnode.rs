use crate::{MindMap, Node};
use quick_xml::de::from_str;
use quick_xml::se::to_string;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Cursor, Read, Write};
use std::time::{SystemTime, UNIX_EPOCH};
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

// MindNode XML Structure (Simplified)
// contents.xml

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename = "mindMap")]
pub struct MindNodeMap {
    #[serde(rename = "document")]
    pub document: MindNodeDocument,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct MindNodeDocument {
    #[serde(rename = "nodes")]
    pub nodes: MindNodeNodes,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct MindNodeNodes {
    #[serde(rename = "node", default)]
    pub node: Vec<MindNodeNode>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct MindNodeNode {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "title")]
    pub title: MindNodeTitle,
    #[serde(rename = "nodes", default)]
    pub children: Option<MindNodeNodes>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct MindNodeTitle {
    #[serde(rename = "text")]
    pub text: String,
}

pub fn to_mindnode(map: &MindMap) -> Result<Vec<u8>, String> {
    let root_node = map.nodes.get(&map.root_id).ok_or("Root node not found")?;

    let mindnode_root = node_to_mindnode_node(root_node, map);

    let mindnode_map = MindNodeMap {
        document: MindNodeDocument {
            nodes: MindNodeNodes {
                node: vec![mindnode_root],
            },
        },
    };

    let xml_content = to_string(&mindnode_map).map_err(|e| e.to_string())?;
    let xml_content = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{}",
        xml_content
    );

    let mut buf = Vec::new();
    let mut zip = ZipWriter::new(Cursor::new(&mut buf));

    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .unix_permissions(0o755);

    zip.start_file("contents.xml", options)
        .map_err(|e| e.to_string())?;
    zip.write_all(xml_content.as_bytes())
        .map_err(|e| e.to_string())?;

    zip.finish().map_err(|e| e.to_string())?;

    Ok(buf)
}

fn node_to_mindnode_node(node: &Node, map: &MindMap) -> MindNodeNode {
    let mut children_vec = Vec::new();
    for child_id in &node.children {
        if let Some(child) = map.nodes.get(child_id) {
            children_vec.push(node_to_mindnode_node(child, map));
        }
    }

    let children = if children_vec.is_empty() {
        None
    } else {
        Some(MindNodeNodes { node: children_vec })
    };

    MindNodeNode {
        id: node.id.clone(),
        title: MindNodeTitle {
            text: node.content.clone(),
        },
        children,
    }
}

pub fn from_mindnode(data: &[u8]) -> Result<MindMap, String> {
    let reader = Cursor::new(data);
    let mut archive = ZipArchive::new(reader).map_err(|e| e.to_string())?;

    let mut xml_content = String::new();
    let mut file = archive
        .by_name("contents.xml")
        .map_err(|_| "contents.xml not found in archive")?;
    file.read_to_string(&mut xml_content)
        .map_err(|e| e.to_string())?;

    let mindnode_map: MindNodeMap = from_str(&xml_content).map_err(|e| e.to_string())?;

    let mut nodes = HashMap::new();
    // MindNode can have multiple top level nodes in the XML structure defined above,
    // but usually one main map. We'll take the first one as root.

    if mindnode_map.document.nodes.node.is_empty() {
        return Ok(MindMap::new());
    }

    let root_id = mindnode_node_to_node(&mindnode_map.document.nodes.node[0], None, &mut nodes);

    Ok(MindMap {
        nodes,
        root_id: root_id.clone(),
        selected_node_id: root_id,
    })
}

fn mindnode_node_to_node(
    mn_node: &MindNodeNode,
    parent_id: Option<&str>,
    nodes: &mut HashMap<String, Node>,
) -> String {
    let id = mn_node.id.clone(); // Use existing ID if possible, or generate new? MindNode IDs are UUIDs usually.
    // If ID is not a valid UUID or we want to ensure uniqueness, we might generate new one.
    // But let's try to use it.

    let mut children_ids = Vec::new();
    if let Some(children) = &mn_node.children {
        for child in &children.node {
            children_ids.push(mindnode_node_to_node(child, Some(&id), nodes));
        }
    }

    let node = Node {
        id: id.clone(),
        content: mn_node.title.text.clone(),
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
    fn test_mindnode_serialization() {
        let mut map = MindMap::new();
        let root_id = map.root_id.clone();
        map.nodes.get_mut(&root_id).unwrap().content = "Root MindNode".to_string();

        map.add_child(&root_id, "Child 1".to_string()).unwrap();

        let data = to_mindnode(&map).unwrap();
        assert!(!data.is_empty());

        let loaded_map = from_mindnode(&data).unwrap();
        let root = loaded_map.nodes.get(&loaded_map.root_id).unwrap();
        assert_eq!(root.content, "Root MindNode");
        assert_eq!(root.children.len(), 1);
    }
}
