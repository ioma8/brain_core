use crate::{MindMap, Node};
use quick_xml::de::from_str;
use quick_xml::se::to_string;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Cursor, Read, Write};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

// MindManager XML Structure (Simplified)
// Usually Document.xml

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename = "ap:Map")]
pub struct MmapMap {
    #[serde(rename = "@xmlns:ap")]
    pub xmlns_ap: String,
    #[serde(rename = "ap:OneTopic", alias = "OneTopic")]
    pub root_topic: MmapTopic,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct MmapTopic {
    #[serde(rename = "ap:Text", alias = "Text")]
    pub text: MmapText,
    #[serde(rename = "ap:SubTopics", alias = "SubTopics", default)]
    pub sub_topics: Option<MmapSubTopics>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct MmapText {
    #[serde(rename = "@PlainText")]
    pub plain_text: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct MmapSubTopics {
    #[serde(rename = "ap:Topic", alias = "Topic", default)]
    pub topics: Vec<MmapTopic>,
}

pub fn to_mmap(map: &MindMap) -> Result<Vec<u8>, String> {
    let root_node = map.nodes.get(&map.root_id).ok_or("Root node not found")?;

    let mmap_root = node_to_mmap_topic(root_node, map);

    let mmap_map = MmapMap {
        xmlns_ap: "http://schemas.mindjet.com/MindManager/Application/2003".to_string(),
        root_topic: mmap_root,
    };

    let xml_content = to_string(&mmap_map).map_err(|e| e.to_string())?;
    let xml_content = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n{}",
        xml_content
    );

    let mut buf = Vec::new();
    let mut zip = ZipWriter::new(Cursor::new(&mut buf));

    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .unix_permissions(0o755);

    zip.start_file("Document.xml", options)
        .map_err(|e| e.to_string())?;
    zip.write_all(xml_content.as_bytes())
        .map_err(|e| e.to_string())?;

    zip.finish().map_err(|e| e.to_string())?;

    Ok(buf)
}

fn node_to_mmap_topic(node: &Node, map: &MindMap) -> MmapTopic {
    let mut sub_topics_vec = Vec::new();
    for child_id in &node.children {
        if let Some(child) = map.nodes.get(child_id) {
            sub_topics_vec.push(node_to_mmap_topic(child, map));
        }
    }

    let sub_topics = if sub_topics_vec.is_empty() {
        None
    } else {
        Some(MmapSubTopics {
            topics: sub_topics_vec,
        })
    };

    MmapTopic {
        text: MmapText {
            plain_text: node.content.clone(),
        },
        sub_topics,
    }
}

pub fn from_mmap(data: &[u8]) -> Result<MindMap, String> {
    let reader = Cursor::new(data);
    let mut archive = ZipArchive::new(reader).map_err(|e| e.to_string())?;

    let mut xml_content = String::new();
    // Try Document.xml, case insensitive if possible, but zip crate is case sensitive usually.
    // MindManager usually uses "Document.xml".

    if let Ok(mut file) = archive.by_name("Document.xml") {
        file.read_to_string(&mut xml_content)
            .map_err(|e| e.to_string())?;
    } else if let Ok(mut file) = archive.by_name("document.xml") {
        file.read_to_string(&mut xml_content)
            .map_err(|e| e.to_string())?;
    } else {
        return Err("Document.xml not found in archive".to_string());
    }

    println!("DEBUG XML: {}", xml_content);

    let mmap_map: MmapMap = from_str(&xml_content).map_err(|e| e.to_string())?;

    let mut nodes = HashMap::new();
    let root_id = mmap_topic_to_node(&mmap_map.root_topic, None, &mut nodes);

    Ok(MindMap {
        nodes,
        root_id: root_id.clone(),
        selected_node_id: root_id,
    })
}

fn mmap_topic_to_node(
    topic: &MmapTopic,
    parent_id: Option<&str>,
    nodes: &mut HashMap<String, Node>,
) -> String {
    let id = Uuid::new_v4().to_string();

    let mut children_ids = Vec::new();
    if let Some(sub) = &topic.sub_topics {
        for child in &sub.topics {
            children_ids.push(mmap_topic_to_node(child, Some(&id), nodes));
        }
    }

    let node = Node {
        id: id.clone(),
        content: topic.text.plain_text.clone(),
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
    fn test_mmap_serialization() {
        let mut map = MindMap::new();
        let root_id = map.root_id.clone();
        map.nodes.get_mut(&root_id).unwrap().content = "Root Mmap".to_string();

        map.add_child(&root_id, "Child 1".to_string()).unwrap();

        let mmap_data = to_mmap(&map).unwrap();
        assert!(!mmap_data.is_empty());

        let loaded_map = from_mmap(&mmap_data).unwrap();
        let root = loaded_map.nodes.get(&loaded_map.root_id).unwrap();
        assert_eq!(root.content, "Root Mmap");
        assert_eq!(root.children.len(), 1);
    }
}
