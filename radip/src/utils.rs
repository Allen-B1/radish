//! Auxiliary functionality not related to adjudicating a movement phase.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{MapState, ProvinceAbbr};

/// Metadata associated to a province.
#[derive(Clone, Serialize, Deserialize)]
pub struct ProvinceMeta {
    pub name: String,
    pub is_sc: bool,
    pub home_sc: String,
}

/// Metadata associated to a map.
#[derive(Clone, Serialize, Deserialize)]
pub struct MapMeta {
    pub name: String,
    pub author: String,

    pub powers: HashMap<String, PowerMeta>,
    pub starting_state: MapState,

    pub provinces: HashMap<ProvinceAbbr, ProvinceMeta>,

    /// Any miscellaneous data.
    pub data: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PowerMeta {
    /// Full name; e.g. England, not ENG.
    pub name: String,

    pub tile_color: String,
    pub sc_color: String,
}