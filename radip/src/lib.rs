//! DATC-compliant\* diplomacy adjucator.
//!
//! Use [`adjudicate`] to adjudicate a movement phase, and [`utils::apply_adjudication`] to update the map.
//! This crate does not support build phases, as those are fairly easy to implement on your own, and
//! build phases may differ for different variants.

use std::{
    any::Any,
    collections::{HashMap, HashSet, VecDeque},
    fmt::Debug,
    ops::Deref,
};

pub mod base;
pub mod core;
pub mod utils;

mod test;
mod paradox;

/// Abbreviation for a province (e.g. NTH, Lvn).
pub type ProvinceAbbr = String;

/// A fleet location. The first element of the tuple is the province, the second is the coast.
pub type FleetLoc = (ProvinceAbbr, String);

/// An army location.
pub type ArmyLoc = ProvinceAbbr;

use serde::{Deserialize, Serialize};

/// Province metadata stored in a [`Map`].
#[derive(Clone, Serialize, Deserialize)]
pub struct Province {
    pub coasts: HashSet<String>,
    pub is_sea: bool,
}

/// A variant map.
#[derive(Clone, Serialize, Deserialize)]
pub struct Map {
    pub provinces: HashMap<ProvinceAbbr, Province>,
    pub fleet_adj: HashSet<(FleetLoc, FleetLoc)>,
    pub army_adj: HashSet<(ArmyLoc, ArmyLoc)>,
}

impl Map {
    /// The classic map.
    pub fn classic() -> Self {
        let default = include_str!("../data/classic.json");

        serde_json::from_str(default).unwrap()
    }
}

/// Stores the units present on a diplomacy board.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MapState {
    pub units: HashMap<String, Unit>,

    /// Tracks SC ownership.
    /// This field is not used in the core adjudicator;
    /// it is included for convenience.
    pub ownership: HashMap<ProvinceAbbr, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "lowercase")]
pub enum Unit {
    Army(String),
    Fleet(String, String),
}

impl Unit {
    pub fn nationality(&self) -> String {
        match self {
            Unit::Army(s) => s.clone(),
            Unit::Fleet(s, _) => s.clone(),
        }
    }
}

/// Map of orders. Each province occupied by a unit have
/// an associated order.
pub type Orders = HashMap<String, Box<dyn Order>>;

/// Helper trait.
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
#[typetag::serde(tag = "type")]
pub trait Order: 'static + Debug + AsAny + Send + Sync {
    /// Return orders (identified by source province) that this order depends on
    /// for resolution.
    fn deps(
        &self,
        map: &Map,
        state: &MapState,
        orders: &Orders,
        this_prov: &str,
    ) -> HashSet<String>;

    /// Based on the given `order_status` information, determine whether
    /// this order succeeds or fails.
    fn adjudicate(
        &self,
        map: &Map,
        state: &MapState,
        orders: &Orders,
        this_prov: &str,
        order_status: &HashMap<String, bool>,
    ) -> Option<bool>;

    fn as_owned(&self) -> Box<dyn Order>;
}

impl Deref for dyn Order {
    type Target = dyn Any;
    fn deref(&self) -> &Self::Target {
        self.as_any()
    }
}

impl Clone for Box<dyn Order> {
    fn clone(&self) -> Self {
        self.as_owned()
    }
}

/// Adjudicate a movement phase.
pub fn adjudicate(map: &Map, state: &MapState, orders: &Orders) -> HashMap<String, bool> {
    let mut order_status: HashMap<String, bool> = HashMap::new();
    loop {
        if order_status.len() == orders.len() {
            break;
        }

        let num_resolved = order_status.len();

        for (prov, order) in orders.iter() {
            if order_status.contains_key(prov) {
                continue;
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
                Some(status) => {
                    order_status.insert(prov.to_string(), status);
                }
                None => {}
            }
        }

        if order_status.len() != num_resolved {
            // skip paradox step if an order was resolved
            continue;
        }

        // paradoxes
    
        paradox::handle_cycles(map, state, orders, &mut order_status);
        if order_status.len() != num_resolved {
            continue;
        }

        paradox::handle_convoy(map, state, orders, &mut order_status);
        if order_status.len() != num_resolved {
            continue;
        }

        break;
    }

    order_status
}
