//! Auxiliary functionality not related to adjudicating a movement phase.

use std::{cmp::min, collections::{HashMap, HashSet}};

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
pub fn apply_adjudication(map: &Map, state: &MapState, orders: &Orders, order_status: &HashMap<String, bool>) -> (MapState, HashMap<String, RetreatOptions>) {
    let mut contested = HashSet::new();
    for (prov, order) in orders.iter() {
        if order.is::<Move>() && base::compute_prevent_strength(map, state, orders, order_status, prov).min != 0 {
            let mov = order.downcast_ref::<Move>().unwrap();
            contested.insert(mov.dest.0.to_string());
        }
    }

    let mut new_state = MapState {
        units: HashMap::new(),
        ownership: state.ownership.clone(),
    };
    let mut retreats = HashMap::new();
    for (prov, status) in order_status.iter() {
        if !(*status && orders[prov].is::<Move>()) {
            new_state.units.insert(prov.clone(), state.units.get(prov).expect("no unit, yet there exists an order").clone());
        }
    }
    for (prov, status) in order_status.iter() {
        if *status && orders[prov].is::<Move>() {
            let mov = orders[prov].downcast_ref::<Move>().unwrap();
            if state.units.contains_key(&mov.dest.0) && !(orders[&mov.dest.0].is::<Move>() && order_status.get(&mov.dest.0) == Some(&true)) {
                retreats.insert(mov.dest.0.clone(), RetreatOptions {
                    src: state.units[&mov.dest.0].clone(),
                    dest: HashSet::new()
                });
            }

            let unit = state.units.get(prov).expect("No unit, yet there exists an order");
            new_state.units.insert(mov.dest.0.clone(), match unit {
                Unit::Army(natl) => Unit::Army(natl.clone()),
                Unit::Fleet(natl, _) => Unit::Fleet(natl.clone(), mov.dest.1.clone())
            });
        }
    }

    for (src_prov, retreat) in retreats.iter_mut() {
        match &retreat.src {
            Unit::Army(natl) => {
                for (src, dest) in map.army_adj.iter() {
                    if src == src_prov && !contested.contains(dest) && !new_state.units.contains_key(dest) {
                        retreat.dest.insert((dest.to_string(), "".to_string()));
                    }
                }
            },
            Unit::Fleet(natl, src_coast) => {
                for (src, dest) in map.fleet_adj.iter() {
                    if src.0 == *src_prov && src.1 == *src_coast && !contested.contains(&dest.0) && !new_state.units.contains_key(&dest.0) {
                        retreat.dest.insert((dest.0.to_string(),dest.1.to_string()));
                    }
                }
            }
        }
    }

    (new_state, retreats)
}


/// Returns the number of units the given power has on the board.
pub fn count_units(state: &MapState, power: &str) -> usize {
    return state.units.values().filter(|u| u.nationality() == power).count();
}

/// Returns the SC count of the given power.
pub fn count_supply(state: &MapState, power: &str) -> usize {
    return state.ownership.values().filter(|u| *u == power).count();
}

/// Decides which unit to disband in the case of civil disorder.
pub fn disband_cd<'a>(map: &Map, state: &MapState, home: impl Iterator<Item=&'a str>, power: &str) -> Option<String> {
    let mut unvisited = map.provinces.keys().map(|c| c.as_str()).collect::<HashSet<_>>();
    let mut dist: HashMap<&str, u32> = HashMap::from_iter(unvisited.iter().map(|c| (*c, u32::MAX)));
    for prov in home {
        dist.insert(prov, 0);
    }
    loop {
        let min = match unvisited.iter().map(|c| dist[c]).min() {
            Some(min) => min,
            None => break
        };
        let node = unvisited.iter().find(|c| dist[*c] == min).map(|c| *c).unwrap();

        for neighbor in map.army_adj.iter().filter(|(a, _)| a == node).map(|(_, dest)| dest.as_str()).chain(
            map.fleet_adj.iter().filter(|(a, _)| a.0 == node).map(|(_, dest)| dest.0.as_str())) {
            dist.insert(neighbor, std::cmp::min(dist[neighbor], min + 1));
        }
    }

    let max: u32 = state.units.iter().filter(|(_, u)| &u.nationality() == power).map(|(prov, _)| dist[prov.as_str()]).min()?;
    state.units.iter()
        .filter(|(prov, u)| &u.nationality() == power && dist[prov.as_str()] == max)
        .map(|(prov, _)| prov).min().map(|c| c.clone())
}