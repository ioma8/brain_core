use crate::{MindMap, Node};
use quick_xml::de::from_str;
use quick_xml::se::to_string;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename = "map")]
pub struct XmlMap {
    #[serde(rename = "@version")]
    pub version: String,
    #[serde(rename = "node")]
    pub root: XmlNode,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename = "icon")]
pub struct XmlIcon {
    #[serde(rename = "@BUILTIN")]
    pub builtin: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename = "node")]
pub struct XmlNode {
    #[serde(rename = "@ID")]
    pub id: String,
    #[serde(rename = "@TEXT")]
    pub text: String,

    #[serde(rename = "@CREATED")]
    pub created: u64,
    #[serde(rename = "@MODIFIED")]
    pub modified: u64,

    #[serde(rename = "@POSITION", skip_serializing_if = "Option::is_none")]
    pub position: Option<String>,

    #[serde(rename = "icon", default)]
    pub icons: Vec<XmlIcon>,

    #[serde(rename = "node", default)]
    pub children: Vec<XmlNode>,
}

pub fn to_xml(map: &MindMap) -> Result<String, String> {
    let root = map.nodes.get(&map.root_id).ok_or("Root not found")?;
    let xml_root = to_xml_node(root, map, true);

    let xml_map = XmlMap {
        version: "1.0.1".to_string(),
        root: xml_root,
    };

    let mut xml = String::from(
        "<!-- To view this file, download free mind mapping software FreeMind from http://freemind.sourceforge.net -->\n",
    );
    xml.push_str(&to_string(&xml_map).map_err(|e| e.to_string())?);
    Ok(xml)
}

fn to_xml_node(node: &Node, map: &MindMap, _is_root: bool) -> XmlNode {
    let mut children = Vec::new();
    for child_id in &node.children {
        if let Some(child_node) = map.nodes.get(child_id) {
            children.push(to_xml_node(child_node, map, false));
        }
    }

    let mut icons = Vec::new();
    for icon_name in &node.icons {
        icons.push(XmlIcon {
            builtin: icon_name.clone(),
        });
    }

    let position = if let Some(parent_id) = &node.parent {
        if parent_id == &map.root_id {
            Some("right".to_string())
        } else {
            None
        }
    } else {
        None
    };

    XmlNode {
        id: node.id.clone(),
        text: node.content.clone(),
        created: node.created,
        modified: node.modified,
        position,
        icons,
        children,
    }
}

pub fn from_xml(xml: &str) -> Result<MindMap, String> {
    let xml_map: XmlMap = from_str(xml).map_err(|e| e.to_string())?;

    let mut nodes = std::collections::HashMap::new();
    let root_id = xml_map.root.id.clone();

    helpers::flatten_nodes(xml_map.root, None, &mut nodes);

    Ok(MindMap {
        nodes,
        root_id: root_id.clone(),
        selected_node_id: root_id,
    })
}

mod helpers {
    use super::*;
    use crate::Node;

    pub fn flatten_nodes(
        xml_node: XmlNode,
        parent_id: Option<String>,
        nodes: &mut std::collections::HashMap<String, Node>,
    ) {
        let node_id = xml_node.id.clone();

        let mut children_ids = Vec::new();
        for child in &xml_node.children {
            children_ids.push(child.id.clone());
        }

        for child in xml_node.children {
            flatten_nodes(child, Some(node_id.clone()), nodes);
        }

        let mut icons = Vec::new();
        for icon in xml_node.icons {
            icons.push(icon.builtin);
        }

        let node = Node {
            id: node_id.clone(),
            content: xml_node.text,
            children: children_ids,
            parent: parent_id,
            x: 0.0,
            y: 0.0,
            created: xml_node.created,
            modified: xml_node.modified,
            icons,
        };

        nodes.insert(node_id, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MindMap;

    #[test]
    fn test_export_import() {
        let mut map = MindMap::new();
        let root_id = map.root_id.clone();

        let child1 = map.add_child(&root_id, "Child 1".to_string()).unwrap();
        let _child2 = map.add_child(&root_id, "Child 2".to_string()).unwrap();
        let _grand1 = map.add_child(&child1, "Grand 1".to_string()).unwrap();

        // Compute layout to have non-zero coords
        map.compute_layout();

        let xml_output = to_xml(&map).expect("Failed to export to XML");

        // Debug output
        println!("XML Output: {}", xml_output);

        let loaded_map = from_xml(&xml_output).expect("Failed to import from XML");

        // Verify structure
        assert_eq!(loaded_map.nodes.len(), map.nodes.len());
        assert!(loaded_map.nodes.contains_key(&root_id));
        assert!(loaded_map.nodes.contains_key(&child1));

        let r_orig = map.nodes.get(&root_id).unwrap();
        let r_load = loaded_map.nodes.get(&root_id).unwrap();

        assert_eq!(r_orig.content, r_load.content);
        assert_eq!(r_orig.children.len(), r_load.children.len());

        // Layout is recomputed on load, or we trust it.
        // Since we don't save X/Y, we can't assert equality unless we recompute layout on both.
        // But map currently has layout computed. loaded_map does NOT have layout computed yet (X/Y=0).
        assert_eq!(r_load.x, 0.0);
        assert_eq!(r_load.y, 0.0);
    }
}
