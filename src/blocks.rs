//"abstractBlock": {
//      "type": "object",
//      "required": ["type"],
//      "properties": {
//        "type": {
//          "type": "string",
//          "const": "block"
//        },
//        "id": {
//          "type": "string"
//        },
//        "title": { "$ref": "#/$defs/inlines" },
//        "reftext": { "$ref": "#/$defs/inlines" },
//        "metadata": { "$ref": "#/$defs/blockMetadata" },
//        "location": { "$ref": "#/$defs/location" }
//      }
//    },
//    "abstractHeading": {
//      "type": "object",
//      "allOf": [{ "$ref": "#/$defs/abstractBlock" }],
//      "required": ["title", "level"],
//      "properties": {
//        "level": {
//          "type": "integer",
//          "minimum": 0
//        }
//      }
//    },
//    "abstractListItem": {
//      "type": "object",
//      "allOf": [{ "$ref": "#/$defs/abstractBlock" }],
//      "required": ["marker"],
//      "defaults": { "blocks": [] },
//      "properties": {
//        "marker": {
//          "type": "string"
//        },
//        "principal": { "$ref": "#/$defs/inlines" },
//        "blocks": { "$ref": "#/$defs/nonSectionBlockBody" }
//      }
//    },

use serde::Serialize;

use crate::nodes::{Location, NodeTypes};

pub enum _ToFindHomesFor {
    SectionBody,
    NonSectionBlockBody,
}

#[derive(Serialize)]
pub enum Block {
    Section, // sort of a special case but prob needs to be included here
    List,
    Dlist,
    DiscreteHeading,
    Break,
    BlockMacro,
    LeafBlock,
    ParentBlock, // Admonitions are hiding in here for some reason
}

#[derive(Serialize)]
pub struct BlockMetadata {}

#[derive(Serialize)]
pub struct List {
    name: String,
    #[serde(rename = "type")]
    node_type: NodeTypes,
    marker: String,
    variant: ListVariant,
    items: Vec<ListItem>,
    location: Vec<Location>,
}

impl List {
    fn new(variant: ListVariant, marker: String, location: Vec<Location>) -> Self {
        List {
            name: "list".to_string(),
            node_type: NodeTypes::Block,
            marker,
            variant,
            items: vec![],
            location,
        }
    }
}

#[derive(Serialize)]
pub enum ListVariant {
    Callout,
    Ordered,
    Unordered,
}

#[derive(Serialize)]
pub struct ListItem {}
