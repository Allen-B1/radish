//! Diplomacy adjucator, based on [Kruijswijk's specification](https://webdiplomacy.net/doc/DATC_v3_0.html).
//! 
//! Use [`adjudicate`] to adjudicate a movement phase, and [`utils::apply_adjudication`] to update the map.
//! This crate does not support build phases, as those are fairly easy to implement on your own, and
//! build phases may differ for different variants.

use std::{any::Any, clone, collections::{HashMap, HashSet}, fmt::Debug, ops::Deref};
pub mod builtin;
mod test;

pub type ProvinceAbbr = String;

pub type FleetLoc = (ProvinceAbbr, String);
pub type ArmyLoc = ProvinceAbbr;

use serde::{Serialize,Deserialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Province {
    pub name: String,
    pub coasts: HashSet<String>,
    pub is_sea: bool
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Map {
    pub provinces: HashMap<ProvinceAbbr, Province>,
    pub fleet_adj: HashSet<(FleetLoc, FleetLoc)>,
    pub army_adj: HashSet<(ArmyLoc, ArmyLoc)>
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MapState {
    pub units: HashMap<String, Unit>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum Unit {
    Army(String),
    Fleet(String, String),
}

impl Unit {
    pub fn nationality(&self) -> String {
        match self {
            Unit::Army(s) => s.clone(),
            Unit::Fleet(s, _) => s.clone()
        }
    }
}

/// Map of orders. Each province occupied by a unit have
/// an associated order.
pub type Orders = HashMap<String, Box<dyn Order>>;

pub trait AsAny {
    fn as_any(&self) -> &dyn Any;
}

impl<T: Any> AsAny for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Represents an order object.
/// 
/// Downcasting is supported via [`Any::downcast_ref`](std::any::Any::downcast_ref) and [`Any::is`](std::any::Any::is).
pub trait Order: 'static + Debug + AsAny {
    /// Return orders (identified by source province) that this order depends on
    /// for resolution.
    fn deps(&self, map: &Map, state: &MapState, orders: &Orders, this_prov: &str) -> HashSet<String>;

    /// Based on the given `order_status` information, determine whether
    /// this order succeeds or fails.
    fn adjudicate(&self, map: &Map, state: &MapState, orders: &Orders, this_prov: &str, order_status: &HashMap<String, bool>) -> Option<bool>;
}

impl Deref for dyn Order {
    type Target = dyn Any;
    fn deref(&self) -> &Self::Target {
        self.as_any()
    }
}

/// Adjudicate a movement phase.
pub fn adjudicate(map: &Map, state: &MapState, orders: &Orders) -> HashMap<String, bool> {
    let mut order_status: HashMap<String, bool> = HashMap::new();
    loop {
        let num_resolved = order_status.len();

        for (prov, order) in orders.iter() {
            if order_status.contains_key(prov) {
                continue
            }

            let deps = order.deps(&map, &state, &orders, prov);
            let mut restricted_order_status = HashMap::new();
            for dep_prov in deps {
                if order_status.contains_key(&dep_prov) {
                    let status = order_status[&dep_prov];
                    restricted_order_status.insert(dep_prov, status);
                }
            }

            match order.adjudicate(&map, &state, &orders, prov, &restricted_order_status) {
                Some(status) => { order_status.insert(prov.to_string(), status); },
                None => {}
            }
        }

        if order_status.len() == orders.len() {
            break
        }

        if order_status.len() != num_resolved {
            // skip paradox step if an order was resolved
            continue
        }

        // cycles & paradoxes
//        panic!("cycle or paradox")
        break
    }

    order_status
}