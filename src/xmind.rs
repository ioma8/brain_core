use serde::{Deserialize, Serialize};
use crate::{MindMap, Node};
use std::io::{Read, Write, Cursor};
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

// XMind JSON structures
#[derive(Debug, Serialize, Deserialize)]
pub struct XmindSheet {
    pub id: String,
    #[serde(rename = "class")]
    pub class_name: Option<String>,
    #[serde(rename = "rootTopic")]
    pub root_topic: XmindTopic,
    pub title: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct XmindTopic {
    pub id: String,
    #[serde(rename = "class")]
    pub class_name: Option<String>,
    pub title: String,
    #[serde(default)]
    pub markers: Vec<XmindMarker>,
    #[serde(default)]
    pub children: Option<XmindChildren>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct XmindMarker {
    #[serde(rename = "markerId")]
    pub marker_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct XmindChildren {
    #[serde(default)]
    pub attached: Vec<XmindTopic>,
}

// Marker ID to FreeMind icon name mapping
fn marker_to_icon(marker_id: &str) -> Option<String> {
    let icon = match marker_id {
        "other-lightbulb" => "idea",
        "other-question" => "help",
        "other-yes" => "yes",
        "other-exclam" => "messagebox_warning",
        "priority-stop" | "other-no" => "stop-sign",
        "priority-1" => "full-1",
        "priority-2" => "full-2",
        "priority-3" => "full-3",
        "priority-4" => "full-4",
        "priority-5" => "full-5",
        "priority-6" => "full-6",
        "priority-7" => "full-7",
        "priority-8" => "full-8",
        "priority-9" => "full-9",
        "smiley-smile" => "ksmiletris",
        "smiley-laugh" => "ksmiletris",
        "smiley-angry" => "smiley-angry",
        "smiley-cry" => "smily_bad",
        "smiley-surprise" => "smiley-oh",
        "task-start" => "go",
        "task-pause" => "prepare",
        "task-done" => "button_ok",
        "flag-red" => "flag",
        "flag-orange" => "flag-orange",
        "flag-yellow" => "flag-yellow",
        "flag-blue" => "flag-blue",
        "flag-green" => "flag-green",
        "flag-purple" => "flag-pink",
        "star-red" | "star-orange" | "star-yellow" | "star-blue" | "star-green" | "star-purple" => "bookmark",
        "people-green" | "people-red" | "people-blue" => "group",
        "arrow-up" => "up",
        "arrow-down" => "down",
        "arrow-left" => "back",
        "arrow-right" => "forward",
        "symbol-info" => "info",
        "symbol-question" => "help",
        "symbol-exclam" => "messagebox_warning",
        "symbol-wrong" => "button_cancel",
        "symbol-right" => "button_ok",
        "symbol-plus" => "yes",
        "symbol-minus" => "closed",
        "c_simbol-attention" => "messagebox_warning",
        _ => return None,
    };
    Some(icon.to_string())
}

// FreeMind icon to XMind marker mapping
fn icon_to_marker(icon: &str) -> String {
    match icon {
        "idea" => "other-lightbulb",
        "help" => "other-question",
        "yes" => "other-yes",
        "messagebox_warning" => "other-exclam",
        "stop-sign" => "priority-stop",
        "closed" => "symbol-minus",
        "info" => "symbol-info",
        "button_ok" => "task-done",
        "button_cancel" => "symbol-wrong",
        "full-1" => "priority-1",
        "full-2" => "priority-2",
        "full-3" => "priority-3",
        "full-4" => "priority-4",
        "full-5" => "priority-5",
        "full-6" => "priority-6",
        "full-7" => "priority-7",
        "full-8" => "priority-8",
        "full-9" => "priority-9",
        "full-0" => "priority-1",
        "go" => "task-start",
        "prepare" => "task-pause",
        "stop" => "priority-stop",
        "back" => "arrow-left",
        "forward" => "arrow-right",
        "up" => "arrow-up",
        "down" => "arrow-down",
        "flag" => "flag-red",
        "flag-black" => "flag-red",
        "flag-blue" => "flag-blue",
        "flag-green" => "flag-green",
        "flag-orange" => "flag-orange",
        "flag-yellow" => "flag-yellow",
        "flag-pink" => "flag-purple",
        "ksmiletris" => "smiley-smile",
        "smiley-angry" => "smiley-angry",
        "smily_bad" => "smiley-cry",
        "smiley-oh" => "smiley-surprise",
        "smiley-neutral" => "smiley-smile",
        "group" => "people-green",
        "bookmark" => "star-yellow",
        _ => "other-question", // fallback
    }.to_string()
}

pub fn from_xmind(data: &[u8]) -> Result<MindMap, String> {
    let cursor = Cursor::new(data);
    let mut archive = ZipArchive::new(cursor).map_err(|e| e.to_string())?;
    
    // Find and read content.json
    let mut content_json = String::new();
    {
        let mut file = archive.by_name("content.json").map_err(|e| e.to_string())?;
        file.read_to_string(&mut content_json).map_err(|e| e.to_string())?;
    }
    
    let sheets: Vec<XmindSheet> = serde_json::from_str(&content_json).map_err(|e| e.to_string())?;
    
    if sheets.is_empty() {
        return Err("No sheets found in XMind file".to_string());
    }
    
    // Use first sheet
    let sheet = &sheets[0];
    let mut nodes = std::collections::HashMap::new();
    let root_id = sheet.root_topic.id.clone();
    
    flatten_xmind_topic(&sheet.root_topic, None, &mut nodes);
    
    Ok(MindMap {
        nodes,
        root_id: root_id.clone(),
        selected_node_id: root_id,
    })
}

fn flatten_xmind_topic(topic: &XmindTopic, parent_id: Option<String>, nodes: &mut std::collections::HashMap<String, Node>) {
    let node_id = topic.id.clone();
    
    // Collect children IDs
    let children_ids: Vec<String> = if let Some(children) = &topic.children {
        children.attached.iter().map(|c| c.id.clone()).collect()
    } else {
        Vec::new()
    };
    
    // Convert markers to icons
    let icons: Vec<String> = topic.markers.iter()
        .filter_map(|m| marker_to_icon(&m.marker_id))
        .collect();
    
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    
    let node = Node {
        id: node_id.clone(),
        content: topic.title.clone(),
        children: children_ids,
        parent: parent_id,
        x: 0.0,
        y: 0.0,
        created: now,
        modified: now,
        icons,
    };
    
    nodes.insert(node_id.clone(), node);
    
    // Recurse into children
    if let Some(children) = &topic.children {
        for child in &children.attached {
            flatten_xmind_topic(child, Some(node_id.clone()), nodes);
        }
    }
}

pub fn to_xmind(map: &MindMap) -> Result<Vec<u8>, String> {
    let root = map.nodes.get(&map.root_id).ok_or("Root not found")?;
    let root_topic = build_xmind_topic(root, map);
    
    let sheet = XmindSheet {
        id: uuid::Uuid::new_v4().to_string(),
        class_name: Some("sheet".to_string()),
        root_topic,
        title: Some(root.content.clone()),
    };
    
    let sheets = vec![sheet];
    let content_json = serde_json::to_string(&sheets).map_err(|e| e.to_string())?;
    
    let metadata = serde_json::json!({
        "dataStructureVersion": "2",
        "creator": {
            "name": "BrainRust",
            "version": "0.1.0"
        }
    });
    let metadata_json = serde_json::to_string(&metadata).map_err(|e| e.to_string())?;
    
    let manifest = serde_json::json!({
        "file-entries": {
            "content.json": {},
            "metadata.json": {}
        }
    });
    let manifest_json = serde_json::to_string(&manifest).map_err(|e| e.to_string())?;
    
    // Create ZIP
    let mut buffer = Vec::new();
    {
        let cursor = Cursor::new(&mut buffer);
        let mut zip = ZipWriter::new(cursor);
        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        
        zip.start_file("content.json", options).map_err(|e| e.to_string())?;
        zip.write_all(content_json.as_bytes()).map_err(|e| e.to_string())?;
        
        zip.start_file("metadata.json", options).map_err(|e| e.to_string())?;
        zip.write_all(metadata_json.as_bytes()).map_err(|e| e.to_string())?;
        
        zip.start_file("manifest.json", options).map_err(|e| e.to_string())?;
        zip.write_all(manifest_json.as_bytes()).map_err(|e| e.to_string())?;
        
        zip.finish().map_err(|e| e.to_string())?;
    }
    
    Ok(buffer)
}

fn build_xmind_topic(node: &Node, map: &MindMap) -> XmindTopic {
    let markers: Vec<XmindMarker> = node.icons.iter()
        .map(|icon| XmindMarker { marker_id: icon_to_marker(icon) })
        .collect();
    
    let children: Vec<XmindTopic> = node.children.iter()
        .filter_map(|child_id| map.nodes.get(child_id))
        .map(|child| build_xmind_topic(child, map))
        .collect();
    
    let children_obj = if children.is_empty() {
        None
    } else {
        Some(XmindChildren { attached: children })
    };
    
    XmindTopic {
        id: node.id.clone(),
        class_name: Some("topic".to_string()),
        title: node.content.clone(),
        markers,
        children: children_obj,
    }
}
