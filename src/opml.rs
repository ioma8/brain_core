use crate::{MindMap, Node};
use quick_xml::de::from_str;
use quick_xml::se::to_string;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename = "opml")]
pub struct Opml {
    #[serde(rename = "@version")]
    pub version: String,
    pub head: OpmlHead,
    pub body: OpmlBody,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct OpmlHead {
    pub title: String,
    #[serde(rename = "dateCreated", skip_serializing_if = "Option::is_none")]
    pub date_created: Option<String>,
    #[serde(rename = "dateModified", skip_serializing_if = "Option::is_none")]
    pub date_modified: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct OpmlBody {
    #[serde(rename = "outline", default)]
    pub outlines: Vec<OpmlOutline>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct OpmlOutline {
    #[serde(rename = "@text")]
    pub text: String,
    #[serde(rename = "@_note", skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(rename = "outline", default)]
    pub children: Vec<OpmlOutline>,
}

pub fn to_opml(map: &MindMap) -> Result<String, String> {
    let root_node = map.nodes.get(&map.root_id).ok_or("Root node not found")?;

    let head = OpmlHead {
        title: root_node.content.clone(),
        date_created: None, // TODO: Format date
        date_modified: None,
    };

    let body = OpmlBody {
        outlines: vec![node_to_outline(root_node, map)],
    };

    let opml = Opml {
        version: "2.0".to_string(),
        head,
        body,
    };

    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str(&to_string(&opml).map_err(|e| e.to_string())?);
    Ok(xml)
}

fn node_to_outline(node: &Node, map: &MindMap) -> OpmlOutline {
    let mut children = Vec::new();
    for child_id in &node.children {
        if let Some(child) = map.nodes.get(child_id) {
            children.push(node_to_outline(child, map));
        }
    }

    OpmlOutline {
        text: node.content.clone(),
        note: None, // Could map to something if we had notes
        children,
    }
}

pub fn from_opml(xml: &str) -> Result<MindMap, String> {
    let opml: Opml = from_str(xml).map_err(|e| e.to_string())?;

    let mut nodes = HashMap::new();
    let mut root_id = String::new();

    // OPML can have multiple top-level outlines in body, but MindMap has one root.
    // If multiple, we create a virtual root. If one, we use it.

    if opml.body.outlines.is_empty() {
        return Ok(MindMap::new());
    }

    if opml.body.outlines.len() == 1 {
        root_id = outline_to_node(&opml.body.outlines[0], None, &mut nodes);
    } else {
        // Create a virtual root using the title
        let root = Node {
            id: Uuid::new_v4().to_string(),
            content: opml.head.title.clone(),
            children: Vec::new(),
            parent: None,
            x: 0.0,
            y: 0.0,
            created: now_millis(),
            modified: now_millis(),
            icons: Vec::new(),
        };
        root_id = root.id.clone();
        nodes.insert(root_id.clone(), root);

        for outline in &opml.body.outlines {
            let child_id = outline_to_node(outline, Some(&root_id), &mut nodes);
            if let Some(root_node) = nodes.get_mut(&root_id) {
                root_node.children.push(child_id);
            }
        }
    }

    Ok(MindMap {
        nodes,
        root_id: root_id.clone(),
        selected_node_id: root_id,
    })
}

fn outline_to_node(
    outline: &OpmlOutline,
    parent_id: Option<&str>,
    nodes: &mut HashMap<String, Node>,
) -> String {
    let id = Uuid::new_v4().to_string();

    let mut children_ids = Vec::new();
    for child in &outline.children {
        children_ids.push(outline_to_node(child, Some(&id), nodes));
    }

    let node = Node {
        id: id.clone(),
        content: outline.text.clone(),
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
    fn test_opml_serialization() {
        let mut map = MindMap::new();
        let root_id = map.root_id.clone();
        map.nodes.get_mut(&root_id).unwrap().content = "Root Topic".to_string();

        let child_id = map.add_child(&root_id, "Child 1".to_string()).unwrap();
        map.add_child(&child_id, "Grandchild 1".to_string())
            .unwrap();
        map.add_child(&root_id, "Child 2".to_string()).unwrap();

        let opml_str = to_opml(&map).unwrap();
        println!("Generated OPML: {}", opml_str);

        let loaded_map = from_opml(&opml_str).unwrap();

        assert_eq!(loaded_map.nodes.len(), 4);
        let root = loaded_map.nodes.get(&loaded_map.root_id).unwrap();
        assert_eq!(root.content, "Root Topic");
        assert_eq!(root.children.len(), 2);
    }

    #[test]
    fn test_opml_deserialization_simple() {
        let xml = r#"
<opml version="2.0">
  <head>
    <title>Simple Map</title>
  </head>
  <body>
    <outline text="Root">
      <outline text="Child 1"/>
      <outline text="Child 2">
        <outline text="Grandchild"/>
      </outline>
    </outline>
  </body>
</opml>
"#;
        let map = from_opml(xml).unwrap();
        let root = map.nodes.get(&map.root_id).unwrap();
        assert_eq!(root.content, "Root");
        assert_eq!(root.children.len(), 2);
    }
}
