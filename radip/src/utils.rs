//! Auxiliary functionality not related to adjudicating a movement phase.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::{base::{self, Move}, Map, MapState, Orders, ProvinceAbbr, Unit};

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

#[derive(Clone, Serialize, Deserialize)]
pub struct RetreatOptions {
    /// The retreating unit in its previous location.
    pub src: Unit,

    /// The available retreat locations.
    pub dest: HashSet<(String, String)>,
}

/// Update the game board based on adjudication results.
pub fn apply_adjudication(map: &Map, state: &mut MapState, orders: &Orders, order_status: &HashMap<String, bool>) -> HashMap<String, RetreatOptions> {
    let mut contested = HashSet::new();
    for (prov, order) in orders.iter() {
        if order.is::<Move>() && base::compute_prevent_strength(map, state, orders, order_status, prov).min != 0 {
            let mov = order.downcast_ref::<Move>().unwrap();
            contested.insert(mov.dest.0.to_string());
        }
    }

    let mut retreats = HashMap::new();
    for (prov, status) in order_status.iter() {
        if *status {
            if let Some(mov) = orders[prov].downcast_ref::<Move>() {
                if state.units.contains_key(&mov.dest.0) {
                    retreats.insert(mov.dest.0.clone(), RetreatOptions {
                        src: state.units[&mov.dest.0].clone(),
                        dest: HashSet::new()
                    });
                }

                let unit = state.units.remove(prov).expect("No unit, yet there exists an order");
                state.units.insert(mov.dest.0.clone(), match unit {
                    Unit::Army(natl) => Unit::Army(natl),
                    Unit::Fleet(natl, _) => Unit::Fleet(natl, mov.dest.1.clone())
                });
            }
        }
    }

    for (src_prov, retreat) in retreats.iter_mut() {
        match &retreat.src {
            Unit::Army(natl) => {
                for (src, dest) in map.army_adj.iter() {
                    if src == src_prov && !contested.contains(dest) && !state.units.contains_key(dest) {
                        retreat.dest.insert((dest.to_string(), "".to_string()));
                    }
                }
            },
            Unit::Fleet(natl, src_coast) => {
                for (src, dest) in map.fleet_adj.iter() {
                    if src.0 == *src_prov && src.1 == *src_coast && !contested.contains(&dest.0) && !state.units.contains_key(&dest.0) {
                        retreat.dest.insert((dest.0.to_string(),dest.1.to_string()));
                    }
                }
            }
        }
    }

    retreats
}