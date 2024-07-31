//! Hold, move, support, and convoy orders, along with associated utilities.

use std::{collections::{HashMap, HashSet}, ops::Deref};
use serde::{Serialize,Deserialize};
use crate::{FleetLoc, Map, MapState, Order, OrderImpl, Orders, Province, ProvinceAbbr, Unit};

#[derive(Clone, Serialize, Deserialize)]
pub struct Hold;

pub fn deps_for_hold(map: &crate::Map, state: &crate::MapState, orders: &crate::Orders, this_prov: &str) -> HashSet<String> {
    let mut deps = HashSet::new();
    for (src2, order2) in orders {
        if let Some(mov) = order2.downcast_ref::<Move>() {
            if mov.dest.0 == this_prov {
                deps.insert(src2.to_string());
            }
        }
    }
    deps
}

pub fn is_dislodged(map: &crate::Map, state: &crate::MapState, orders: &crate::Orders, this_prov: &str, order_status: &std::collections::HashMap<String, bool>) -> Option<bool> {
    let mut possible_dislodge = false;
    for (src2, order2) in orders {
        if Move::is_move_to(order2.deref(), this_prov) {
            match order_status.get(src2) {
                Some(true) => return Some(false),
                Some(false) => {}
                None => { possible_dislodge = true }
            }
        }
    }
    
    if possible_dislodge {
        None
    } else {
        Some(true)
    }
}

impl OrderImpl for Hold {
    fn deps(&self, map: &crate::Map, state: &crate::MapState, orders: &crate::Orders, this_prov: &str) -> HashSet<String> {
        deps_for_hold(map, state, orders, this_prov)
    }

    fn adjudicate(&self, map: &crate::Map, state: &crate::MapState, orders: &crate::Orders, this_prov: &str, order_status: &std::collections::HashMap<String, bool>) -> Option<bool> {
        is_dislodged(map, state, orders, this_prov, order_status)
    }
}

#[derive(PartialEq, Clone, Serialize, Deserialize)]
pub struct Move {
    pub dest: FleetLoc
}

impl Move {
    pub fn is_to(&self, prov: &str) -> bool {
        return self.dest.0 == prov
    }

    pub fn is_move_to(this: &dyn Order, prov: &str) -> bool {
        return this.downcast_ref::<Move>().map(|m| m.is_to(prov)).unwrap_or(false);
    }
}

#[derive(Clone)]
pub struct Bounds {
    pub min: u32,
    pub max: u32,
}

impl OrderImpl for Move {
    fn deps(&self, map: &crate::Map, state: &crate::MapState, orders: &crate::Orders, this_prov: &str) -> HashSet<String> {
        let mut deps = HashSet::new();
        for (src2, order2) in orders {
            if let Some(sup) = order2.downcast_ref::<SupportMove>() {
                if sup.dest == self.dest.0 && orders.contains_key(&sup.src) && Move::is_move_to(orders[&sup.src].deref(), &sup.dest) {
                    deps.insert(src2.to_string());
                }
            }
            if let Some(sup) = order2.downcast_ref::<Convoy>() {
                if sup.dest == self.dest.0 && orders.contains_key(&sup.src) && Move::is_move_to(orders[&sup.src].deref(), &sup.dest) {
                    deps.insert(src2.to_string());
                }
            }
            if let Some(sup) = order2.downcast_ref::<SupportHold>() {
                if sup.target == self.dest.0 && orders.contains_key(&sup.target) && !orders[&sup.target].is::<Move>() {
                    deps.insert(src2.to_string());
                }
            }
        }

        if orders.contains_key(&self.dest.0) && orders[&self.dest.0].is::<Move>() {
            deps.insert(self.dest.0.clone());
        }
        deps
    }

    fn adjudicate(&self, map: &crate::Map, state: &crate::MapState, orders: &crate::Orders, this_prov: &str, order_status: &std::collections::HashMap<String, bool>) -> Option<bool> {
        let attack_strength = compute_attack_strength(map, state, orders, order_status, this_prov);

        let mut strengths: Vec<Bounds> = Vec::new();
        if orders.contains_key(&self.dest.0) && Move::is_move_to(orders[&self.dest.0].deref(), this_prov) {
            // TODO: don't think this works?
            if is_convoy_path(map, state, orders, order_status, this_prov) != Some(true) {
                // head-to-head battle
                strengths.push(compute_defend_strength(map, state, orders, order_status, self.dest.0.as_str()));
            }
        } else {
            strengths.push(compute_hold_strength(map, state, orders, order_status, self.dest.0.as_str()));
            // not head-to-head battle
        }

        for (province2, order2) in orders.iter() {
            if Move::is_move_to(order2.deref(), self.dest.0.as_str()) {
                strengths.push(compute_prevent_strength(map, state, orders, order_status, province2.as_str()));
            }
        }

        let mut beats_all_bounds = true;
        for strength in strengths.iter() {
            if attack_strength.max <= strength.min {
                return Some(false);
            }
            if attack_strength.min <= strength.max {
                beats_all_bounds = false;
            }    
        }
        if beats_all_bounds {
            Some(true)
        } else {
            None
        }
    }
}

#[derive(PartialEq, Clone, Serialize, Deserialize)]
pub struct SupportHold {
    pub target: ProvinceAbbr
}

pub fn deps_for_tap(map: &Map, state: &MapState, orders: &Orders, this_prov: &str) -> HashSet<String> {
    let mut deps = HashSet::new();
    for (prov, order) in orders.iter() {
        if let Some(sup) = order.downcast_ref::<Convoy>() {
            if sup.dest == this_prov && orders.contains_key(&sup.src) && Move::is_move_to(orders[&sup.src].deref(), &sup.dest) {
                deps.insert(prov.to_string());
            }
        }
    }
    deps
}

pub fn is_untapped(map: &Map, state: &MapState, orders: &Orders, order_status: &HashMap<String, bool>, this_prov: &str) -> Option<bool> {
    let mut possibly_tapped = false;
    for (prov, order) in orders.iter() {
        if Move::is_move_to(order.deref(), this_prov) {
            match is_path(map, state, orders, order_status, this_prov) {
                Some(true) => return Some(false),
                None => { possibly_tapped = true }
                Some(false) => {}
            }
        }
    }   

    if possibly_tapped {
        None
    } else {
        Some(true)
    }
}

impl OrderImpl for SupportHold {
    fn deps(&self, map: &crate::Map, state: &crate::MapState, orders: &crate::Orders, this_prov: &str) -> HashSet<String> {
        if !orders.contains_key(&self.target) || orders[&self.target].is::<Move>() {
            HashSet::new()
        } else {
            deps_for_tap(map, state, orders, this_prov)
        }
    }

    fn adjudicate(&self, map: &Map, state: &MapState, orders: &Orders, this_prov: &str, order_status: &HashMap<String, bool>) -> Option<bool> {
        if !orders.contains_key(&self.target) || orders[&self.target].is::<Move>() {
            Some(false)
        } else {
            is_untapped(map, state, orders, order_status, this_prov)
        }
    }
}

#[derive(PartialEq, Clone, Serialize, Deserialize)]
pub struct SupportMove {
    pub src: ProvinceAbbr,
    pub dest: ProvinceAbbr
}

impl SupportMove {
    pub fn is_support_to(this: &dyn Order, src: &str, dest: &str) -> bool {
        this.downcast_ref::<SupportMove>().map(|s| s.src == src && s.dest == dest).unwrap_or(false)
    }
}

impl OrderImpl for SupportMove {
    fn deps(&self, map: &Map, state: &MapState, orders: &Orders, this_prov: &str) -> HashSet<String> {
        if !orders.contains_key(&self.src) || !Move::is_move_to(orders[&self.src].deref(), &self.dest) {
            HashSet::new()
        } else {
            deps_for_tap(map, state, orders, this_prov)
        }
    }

    fn adjudicate(&self, map: &Map, state: &MapState, orders: &Orders, this_prov: &str, order_status: &HashMap<String, bool>) -> Option<bool> {
        if !orders.contains_key(&self.src) || !Move::is_move_to(orders[&self.src].deref(), &self.dest) {
            Some(false)
        } else {
            is_untapped(map, state, orders, order_status, this_prov)
        }
    }
}

#[derive(PartialEq, Clone, Serialize, Deserialize)]
pub struct Convoy {
    pub src: ProvinceAbbr,
    pub dest: ProvinceAbbr
}

impl OrderImpl for Convoy {
    fn deps(&self, map: &crate::Map, state: &crate::MapState, orders: &crate::Orders, this_prov: &str) -> HashSet<String> {
        deps_for_hold(map, state, orders, this_prov)
    }

    fn adjudicate(&self, map: &crate::Map, state: &crate::MapState, orders: &crate::Orders, this_prov: &str, order_status: &std::collections::HashMap<String, bool>) -> Option<bool> {
        is_dislodged(map, state, orders, this_prov, order_status)
    }
}

// compute attack, defend, and prevent strengths

/// Compute the [defend strength](https://webdiplomacy.net/doc/DATC_v3_0.html#5.B.7) of the given move order.
pub fn compute_defend_strength(map: &Map, state: &MapState, orders: &Orders, order_status: &HashMap<String, bool>, src: &str) -> Bounds {
    let possibly_no_path = match is_path(map, state, orders, order_status, src) {
        Some(false) => return Bounds { min : 0, max : 0 },
        Some(true) => false,
        None => true
    };

    let (dest_prov, dest_coast) = &orders[src].downcast_ref::<Move>().expect("compute_defend_strength computed without move order").dest;

    let mut bounds = Bounds { min : 1, max : 1 };
    for (province2, order2) in orders.iter() {
        if SupportMove::is_support_to(order2.deref(), &src, &dest_prov) {
            match order_status.get(province2) {
                Some(true) => {
                    bounds.min += 1;
                    bounds.max += 1;
                },
                Some(false) => {},
                None => {
                    bounds.max += 1;
                }
            }
        }
    }

    if possibly_no_path {
        bounds.min = 0;
    }

    bounds
}

/// Compute the [prevent strength](https://webdiplomacy.net/doc/DATC_v3_0.html#5.B.6) of the given move order.
pub fn compute_prevent_strength(map: &Map, state: &MapState, orders: &Orders, order_status: &HashMap<String, bool>, src: &str) -> Bounds {
    compute_defend_strength(map, state, orders, order_status, src)
}

/// Compute the [attack strength](https://webdiplomacy.net/doc/DATC_v3_0.html#5.B.8) of the given move order.
pub fn compute_attack_strength(map: &Map, state: &MapState, orders: &Orders, order_status: &HashMap<String, bool>, src: &str) -> Bounds {
    let possibly_no_path = match is_path(map, state, orders, order_status, src) {
        Some(false) => return Bounds { min : 0, max : 0 },
        Some(true) => false,
        None => true
    };

    let (dest_prov, dest_coast) = &orders[src].downcast_ref::<Move>().expect("src not a move order in compute_attack_strength").dest;

    let mut possible_dest_nationality: Option<String> = state.units.get(dest_prov).map(|x| x.nationality());
    let dest_nationality_certain = if orders.contains_key(dest_prov) && orders[dest_prov].is::<Move>() {
        match order_status.get(dest_prov) {
            None => false,
            Some(true) => {
                possible_dest_nationality = None;
                true
            },
            Some(false) => true
        }
    } else {
        true
    };

    let src_nationality = state.units.get(src).map(Unit::nationality).expect("empty unit in source of move order");

    if possible_dest_nationality == Some(src_nationality.clone()) && dest_nationality_certain {
        return Bounds { min : 0, max : 0 }
    }

    let mut bounds = Bounds { min : 1, max : 1 };
    for (prov_it, order_it) in orders.iter() {
        let support = match order_it.downcast_ref::<SupportMove>() {
            Some(s) => s,
            None => continue
        };

        if !(support.src == src && support.dest == *dest_prov) {
            continue
        }

        match order_status.get(prov_it) {
            Some(true) => {
                // if the destination nationality is definitely not the same as the supporter nationality, increase min + max by 1
                if possible_dest_nationality != Some(state.units[prov_it].nationality()) {
                    bounds.min += 1;
                    bounds.max += 1;
                }
                // if the destination nationality is possibly the same as the supporter nationality, increase max by 1
                else if possible_dest_nationality == Some(state.units[prov_it].nationality()) && !dest_nationality_certain {
                    bounds.max += 1;
                }
                // otherwise (if destination nationality is definitely the same as supporter nationality), support guaranteed to not count
            },
            Some(false) => {},
            None => {
                if !dest_nationality_certain || possible_dest_nationality != Some(state.units[prov_it].nationality()) {
                    bounds.max += 1;
                }
            }
        }
    }

    if possibly_no_path || possible_dest_nationality == Some(src_nationality) {
        bounds.min = 0;
    }

    bounds
}

/// Compute the [hold strength](https://webdiplomacy.net/doc/DATC_v3_0.html#5.B.5) of the target province.
pub fn compute_hold_strength(map: &Map, state: &MapState, orders: &Orders, order_status: &HashMap<String, bool>, target: &str) -> Bounds {
    if !state.units.contains_key(target) {
        return Bounds { max : 0, min : 0 };
    }

    if orders[target].is::<Move>() {
        // supports don't work
        match order_status.get(target) { 
            Some(true) => return Bounds { max : 0, min : 0} ,
            Some(false) => return Bounds { max : 1, min : 1 },
            None => return Bounds { max : 1, min : 0 }
        }
    }

    let mut bounds = Bounds { min : 1, max : 1 };
    for (prov_it, order_it) in orders.iter() {
        let support = match order_it.downcast_ref::<SupportHold>() { 
            Some(s) => s,
            None => continue
        };

        if support.target != target {
            continue
        };

        match order_status.get(prov_it) { 
            Some(true) => { 
                bounds.min += 1;
                bounds.max += 1;
            },
            None => {
                bounds.min += 0;
                bounds.max += 1;
            },
            Some(false) => {
                bounds.min += 0;
                bounds.max += 0;
            }
        }
    }
    bounds
}


pub fn is_direct_path(map: &Map, state: &MapState, orders: &Orders, src: &str) -> bool {
    let (dest_prov, dest_coast) = &orders[src].downcast_ref::<Move>().expect("is_direct_path should have move order").dest;

    match state.units.get(src).expect("unit does not exist in is_path") {
        Unit::Army(_) => map.army_adj.contains(&(src.to_string(), dest_prov.to_string())),
        Unit::Fleet(_, src_coast) => {
            if !(map.provinces[dest_prov].coasts.contains(dest_coast) || (dest_coast == "" && map.provinces[dest_prov].coasts.is_empty())) {
                return false
            }
            map.fleet_adj.contains(&((src.to_string(), src_coast.to_string()), (dest_prov.to_string(), dest_coast.to_string())))
        }
    }
}

fn is_path_along(map: &Map, src: &str, dest: &str, convoys: &[String]) -> bool {
    let mut visited = HashSet::new();
    let mut stack = vec![src.to_string()];
    loop { 
        let node = match stack.pop() {
            Some(n) => n,
            None => return false
        };

        if map.fleet_adj.contains(&((node.to_string(), "".to_string()), (dest.to_string(), "".to_string()))) {
            return true
        }

        for convoy in convoys.iter() {
            if visited.contains(convoy) || node == *convoy {
                continue
            }
            if map.fleet_adj.contains(&((node.to_string(), "".to_string()), (convoy.to_string(), "".to_string()))) {
                stack.push(convoy.to_string());
            }
        }

        visited.insert(node);
    }
}

pub fn is_convoy_path(map: &Map, state: &MapState, orders: &Orders, order_status: &HashMap<String, bool>, src: &str) -> Option<bool> {
    let (dest_prov, dest_coast) = &orders[src].downcast_ref::<Move>().expect("is_convoy_path should have move order").dest;

    match state.units.get(src).expect("unit does not exist in is_path") {
        Unit::Fleet(_, _) => return Some(false),
        _ => {}
    }

    let mut possible_convoys = Vec::new();
    let mut definite_convoys = Vec::new();
    for (prov_it, order_it) in orders.iter() {
        let convoy = match order_it.downcast_ref::<Convoy>() {
            Some(c) => c,
            None => continue
        };

        if !(convoy.src == src && convoy.dest == *dest_prov) {
            continue
        }

        match order_status.get(prov_it) {
            Some(true) => {
                possible_convoys.push(prov_it.to_string());
                definite_convoys.push(prov_it.to_string());
            },
            None => {
                possible_convoys.push(prov_it.to_string());
            },
            Some(false) => {}
        }
    }

    let is_definite = is_path_along(map, src, dest_prov, &definite_convoys);
    if is_definite { 
        return Some(true)
    }
    let is_possible = is_path_along(map, src, dest_prov, &possible_convoys);
    if is_possible {
        return None
    } else {
        return Some(false)
    }
}

pub fn is_path(map: &Map, state: &MapState, orders: &Orders, order_status: &HashMap<String, bool>, src: &str) -> Option<bool> {
    if is_direct_path(map, state, orders, src) {
        return Some(true)
    }

    is_convoy_path(map, state, orders, order_status, src)
}