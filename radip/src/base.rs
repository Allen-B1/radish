//! Orders present in the default version of the game.
//! These implementations follow Kruijswijk's specifications in [his excellent article](https://diplom.org/Zine/S2009M/Kruijswijk/DipMath_Chp2.htm)
//! on adjudication.

use crate::{FleetLoc, Map, MapState, Order, Orders, Province, ProvinceAbbr, Unit};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    ops::Deref,
};

/// A hold order. Succeeds if not dislodged.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Hold;

pub fn deps_for_hold(
    map: &crate::Map,
    state: &crate::MapState,
    orders: &crate::Orders,
    this_prov: &str,
) -> HashSet<String> {
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

pub fn is_dislodged(
    map: &crate::Map,
    state: &crate::MapState,
    orders: &crate::Orders,
    this_prov: &str,
    order_status: &std::collections::HashMap<String, bool>,
) -> Option<bool> {
    let mut possible_dislodge = false;
    for (src2, order2) in orders {
        if Move::is_move_to(order2.deref(), this_prov) {
            match order_status.get(src2) {
                Some(true) => return Some(false),
                Some(false) => {}
                None => possible_dislodge = true,
            }
        }
    }

    if possible_dislodge {
        None
    } else {
        Some(true)
    }
}

#[typetag::serde]
impl Order for Hold {
    fn deps(
        &self,
        map: &crate::Map,
        state: &crate::MapState,
        orders: &crate::Orders,
        this_prov: &str,
    ) -> HashSet<String> {
        deps_for_hold(map, state, orders, this_prov)
    }

    fn adjudicate(
        &self,
        map: &crate::Map,
        state: &crate::MapState,
        orders: &crate::Orders,
        this_prov: &str,
        order_status: &std::collections::HashMap<String, bool>,
    ) -> Option<bool> {
        is_dislodged(map, state, orders, this_prov, order_status)
    }
}

/// A move order.
/// 
/// The move order succeeds iff the attack strength is greater than
/// the hold strength of the opposing unit and the prevent strength of all other units
/// moving to the same destination.
/// In the case of a head-to-head battle, the attack strength must also
/// be greater than the defend strength of the opposing unit.
#[derive(PartialEq, Clone, Serialize, Deserialize, Debug)]
pub struct Move {
    pub dest: FleetLoc,
}

impl Move {
    pub fn is_to(&self, prov: &str) -> bool {
        return self.dest.0 == prov;
    }

    pub fn is_move_to(this: &dyn Order, prov: &str) -> bool {
        return this
            .downcast_ref::<Move>()
            .map(|m| m.is_to(prov))
            .unwrap_or(false);
    }
}

/// Compute whether the given order is in a head-to-head battle.
pub fn is_head_to_head(
    map: &crate::Map,
    state: &crate::MapState,
    orders: &crate::Orders,
    order_status: &std::collections::HashMap<String, bool>,
    src: &str,
) -> Option<bool> {
    let (dest_prov, _) = &orders[src].downcast_ref::<Move>().expect("is_head_to_head not given move order").dest;
    
    if !(orders.contains_key(dest_prov) && Move::is_move_to(orders[dest_prov].deref(), src)) {
        return Some(false)
    }

    match (is_convoy_path(map, state, orders, order_status, src), is_convoy_path(map, state, orders, order_status, dest_prov)) {
        (Some(true), _) | (_, Some(true)) => Some(false),
        (None, _) | (_, None) => None,
        (Some(false), Some(false)) => Some(true)
    }
}

#[derive(Clone, Debug)]
pub struct Bounds {
    pub min: u32,
    pub max: u32,
}

#[typetag::serde]
impl Order for Move {
    fn deps(
        &self,
        map: &crate::Map,
        state: &crate::MapState,
        orders: &crate::Orders,
        this_prov: &str,
    ) -> HashSet<String> {
        if this_prov == self.dest.0 { // check is required for convoys!!
            return HashSet::new();
        }

        let mut deps = HashSet::new();
        for (src2, order2) in orders {
            if let Some(sup) = order2.downcast_ref::<SupportMove>() {
                if sup.dest == self.dest.0
                    && orders.contains_key(&sup.src)
                    && Move::is_move_to(orders[&sup.src].deref(), &sup.dest)
                {
                    deps.insert(src2.to_string());
                }
            }
            if let Some(sup) = order2.downcast_ref::<Convoy>() {
                if (sup.dest == self.dest.0 || sup.src == self.dest.0) // second clause required to compute whether self.dest is part of head-to-head battle
                    && orders.contains_key(&sup.src)
                    && Move::is_move_to(orders[&sup.src].deref(), &sup.dest)
                {
                    deps.insert(src2.to_string());
                }
            }
            if let Some(sup) = order2.downcast_ref::<SupportHold>() {
                if sup.target == self.dest.0
                    && orders.contains_key(&sup.target)
                    && !orders[&sup.target].is::<Move>()
                {
                    deps.insert(src2.to_string());
                }
            }
        }

        if orders.contains_key(&self.dest.0) && orders[&self.dest.0].is::<Move>() {
            deps.insert(self.dest.0.clone());

            if Move::is_move_to(orders[&self.dest.0].deref(), this_prov) {
                // possible head-to-head
                for (src2, order2) in orders {
                    if let Some(sup) = order2.downcast_ref::<SupportMove>() {
                        if sup.src == self.dest.0 && sup.dest == this_prov {
                            deps.insert(src2.to_string());
                        }
                    }
                }
            }
        }
        deps
    }

    fn adjudicate(
        &self,
        map: &crate::Map,
        state: &crate::MapState,
        orders: &crate::Orders,
        this_prov: &str,
        order_status: &std::collections::HashMap<String, bool>,
    ) -> Option<bool> {
        if this_prov == self.dest.0 { // check is required for convoys!!
            return Some(false)
        }

        let attack_strength = compute_attack_strength(map, state, orders, order_status, this_prov);

        let mut strengths: Vec<Bounds> = Vec::new();
        if orders.contains_key(&self.dest.0)
            && Move::is_move_to(orders[&self.dest.0].deref(), this_prov)
        {
            let is_hth = is_head_to_head(map, state, orders, order_status, this_prov);
            match is_hth {
                Some(false) => {},
                None => {
                    let mut def = compute_defend_strength(map, state, orders, order_status, self.dest.0.as_str());
                    def.min = 0;
                    strengths.push(def);   
                },
                Some(true) => {
                    let def = compute_defend_strength(map, state, orders, order_status, self.dest.0.as_str());
                    strengths.push(def);
                }
            }

            strengths.push(compute_hold_strength(
                map,
                state,
                orders,
                order_status,
                self.dest.0.as_str(),
            ));
        } else {
            strengths.push(compute_hold_strength(
                map,
                state,
                orders,
                order_status,
                self.dest.0.as_str(),
            ));
            // not head-to-head battle
        }

        for (province2, order2) in orders.iter() {
            if Move::is_move_to(order2.deref(), self.dest.0.as_str()) && province2 != this_prov {
                strengths.push(compute_prevent_strength(
                    map,
                    state,
                    orders,
                    order_status,
                    province2.as_str(),
                ));
            }
        }

        let mut beats_all_bounds = true;
        println!(
            "{} | attack: {:?} | defends: {:?}",
            this_prov, attack_strength, strengths
        );
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

/// A support hold order.
/// The coast is not specified; only the province.
/// 
/// Succeeds iff
/// * the supporting unit is adjacent to the target,
/// * the target does not move, and
/// * the supporting unit is not tapped.
#[derive(PartialEq, Clone, Serialize, Deserialize, Debug)]
pub struct SupportHold {
    pub target: ProvinceAbbr,
}

pub fn deps_for_tap(
    map: &Map,
    state: &MapState,
    orders: &Orders,
    this_prov: &str,
) -> HashSet<String> {
    let mut deps = HashSet::new();
    for (prov_it, order_it) in orders.iter() {
        if Move::is_move_to(order_it.deref(), this_prov) {
            deps.insert(prov_it.to_string());
        }
        if let Some(sup) = order_it.downcast_ref::<Convoy>() {
            if sup.dest == this_prov
                && orders.contains_key(&sup.src)
                && Move::is_move_to(orders[&sup.src].deref(), &sup.dest)
            {
                deps.insert(prov_it.to_string());
            }
        }
    }
    deps
}

pub fn is_untapped(
    map: &Map,
    state: &MapState,
    orders: &Orders,
    order_status: &HashMap<String, bool>,
    this_prov: &str,
    exception: &str,
) -> Option<bool> {
    let mut possibly_tapped = false;
    for (prov_it, order_it) in orders.iter() {
        if Move::is_move_to(order_it.deref(), this_prov) {
            if state.units[prov_it].nationality() == state.units[this_prov].nationality() {
                continue;
            }

            if prov_it == exception {
                match order_status.get(prov_it).map(|x| *x) {
                    Some(true) => return Some(false),
                    None => possibly_tapped = true,
                    Some(false) => {}
                }
            } else {
                match is_path(map, state, orders, order_status, prov_it) {
                    Some(true) => return Some(false),
                    None => possibly_tapped = true,
                    Some(false) => {}
                }
            }
        }
    }

    if possibly_tapped {
        None
    } else {
        Some(true)
    }
}

#[typetag::serde]
impl Order for SupportHold {
    fn deps(
        &self,
        map: &crate::Map,
        state: &crate::MapState,
        orders: &crate::Orders,
        this_prov: &str,
    ) -> HashSet<String> {
        if !orders.contains_key(&self.target)
            || orders[&self.target].is::<Move>()
            || !unit_can_reach(map, state, this_prov, &self.target)
        {
            HashSet::new()
        } else {
            deps_for_tap(map, state, orders, this_prov)
        }
    }

    fn adjudicate(
        &self,
        map: &Map,
        state: &MapState,
        orders: &Orders,
        this_prov: &str,
        order_status: &HashMap<String, bool>,
    ) -> Option<bool> {
        if !orders.contains_key(&self.target)
            || orders[&self.target].is::<Move>()
            || !unit_can_reach(map, state, this_prov, &self.target)
        {
            Some(false)
        } else {
            is_untapped(map, state, orders, order_status, this_prov, "")
        }
    }
}

/// A support move order.
/// The coast of both the source and destination is not specified; only the province.
/// 
/// Succeeds iff
/// * the supporting unit is adjacent to the destination,
/// * the source moves to the destination, and
/// * the supporting unit is not tapped by a unit other than the destination.
#[derive(PartialEq, Clone, Serialize, Deserialize, Debug)]
pub struct SupportMove {
    pub src: ProvinceAbbr,
    pub dest: ProvinceAbbr,
}

impl SupportMove {
    pub fn is_support_to(this: &dyn Order, src: &str, dest: &str) -> bool {
        this.downcast_ref::<SupportMove>()
            .map(|s| s.src == src && s.dest == dest)
            .unwrap_or(false)
    }
}

fn unit_can_reach(map: &Map, state: &MapState, src: &str, dest: &str) -> bool {
    match &state.units[src] {
        Unit::Army(_) => map.army_adj.contains(&(src.to_string(), dest.to_string())),
        Unit::Fleet(_, coast) => map.fleet_adj.contains(&(
            (src.to_string(), coast.to_string()),
            (dest.to_string(), "".to_string()),
        )),
    }
}

#[typetag::serde]
impl Order for SupportMove {
    fn deps(
        &self,
        map: &Map,
        state: &MapState,
        orders: &Orders,
        this_prov: &str,
    ) -> HashSet<String> {
        if !orders.contains_key(&self.src)
            || !Move::is_move_to(orders[&self.src].deref(), &self.dest)
            || !unit_can_reach(map, state, this_prov, &self.dest)
            || self.src == self.dest
        {
            HashSet::new()
        } else {
            deps_for_tap(map, state, orders, this_prov)
        }
    }

    fn adjudicate(
        &self,
        map: &Map,
        state: &MapState,
        orders: &Orders,
        this_prov: &str,
        order_status: &HashMap<String, bool>,
    ) -> Option<bool> {
//        println!("unit {} can {}: {}", this_prov, self.dest, unit_can_reach(map, state, this_prov, &self.dest));

        if !orders.contains_key(&self.src)
            || !Move::is_move_to(orders[&self.src].deref(), &self.dest)
            || !unit_can_reach(map, state, this_prov, &self.dest)
            || self.src == self.dest
        {
            Some(false)
        } else {
            is_untapped(map, state, orders, order_status, this_prov, &self.dest)
        }
    }
}

/// A convoy order.
/// 
/// Succeeds iff the convoying unit
/// * is a fleet
/// * in a sea tile
/// * and is not dislodged.
#[derive(PartialEq, Clone, Serialize, Deserialize, Debug)]
pub struct Convoy {
    pub src: ProvinceAbbr,
    pub dest: ProvinceAbbr,
}

#[typetag::serde]
impl Order for Convoy {
    fn deps(
        &self,
        map: &crate::Map,
        state: &crate::MapState,
        orders: &crate::Orders,
        this_prov: &str,
    ) -> HashSet<String> {
        if !map.provinces[this_prov].is_sea {
            return HashSet::new()
        }
        deps_for_hold(map, state, orders, this_prov)
    }

    fn adjudicate(
        &self,
        map: &crate::Map,
        state: &crate::MapState,
        orders: &crate::Orders,
        this_prov: &str,
        order_status: &std::collections::HashMap<String, bool>,
    ) -> Option<bool> {
        if !map.provinces[this_prov].is_sea {
            return Some(false)
        }
        is_dislodged(map, state, orders, this_prov, order_status)
    }
}

// compute attack, defend, and prevent strengths

/// Compute the [defend strength](https://webdiplomacy.net/doc/DATC_v3_0.html#5.B.7) of the given move order.
/// 
/// If the path of the move order is not successful, then the defend strength is 0.  
/// Otherwise, the defend strength is 1 + the number of successful support orders.
pub fn compute_defend_strength(
    map: &Map,
    state: &MapState,
    orders: &Orders,
    order_status: &HashMap<String, bool>,
    src: &str,
) -> Bounds {
    let possibly_no_path = match is_path(map, state, orders, order_status, src) {
        Some(false) => return Bounds { min: 0, max: 0 },
        Some(true) => false,
        None => true,
    };

    let (dest_prov, dest_coast) = &orders[src]
        .downcast_ref::<Move>()
        .expect("compute_defend_strength computed without move order")
        .dest;

    let mut bounds = Bounds { min: 1, max: 1 };
    for (province2, order2) in orders.iter() {
        if SupportMove::is_support_to(order2.deref(), &src, &dest_prov) {
            match order_status.get(province2) {
                Some(true) => {
                    bounds.min += 1;
                    bounds.max += 1;
                }
                Some(false) => {}
                None => {
//                    println!("possible support for {}: {}", src, province2);
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
/// 
/// If the move is part of a head-to-head battle and the opposing move is successful, the prevent strength is 0.
/// If the path of the move order is not successful, then the defend strength is 0.  
/// Otherwise, the defend strength is 1 + the number of successful support orders.
pub fn compute_prevent_strength(
    map: &Map,
    state: &MapState,
    orders: &Orders,
    order_status: &HashMap<String, bool>,
    src: &str,
) -> Bounds {
    let mut bounds = compute_defend_strength(map, state, orders, order_status, src);
    
    let (dest_prov, dest_coast) = &orders[src]
        .downcast_ref::<Move>()
        .expect("compute_defend_strength computed without move order")
        .dest;

    match is_head_to_head(map, state, orders, order_status, src) {
        Some(false) => {},
        None => {
            if order_status.get(dest_prov).map(|x: &bool| *x) != Some(false) {
                bounds.min = 0;
            }
        },
        Some(true) => {
            if order_status.get(dest_prov).map(|x| *x) != Some(false) {
                bounds.min = 0;
            }
            if order_status.get(dest_prov).map(|x| *x) == Some(true) {
                bounds.max = 0;
            }
        }
    }

    bounds
}

/// Compute the [attack strength](https://webdiplomacy.net/doc/DATC_v3_0.html#5.B.8) of the given move order.
pub fn compute_attack_strength(
    map: &Map,
    state: &MapState,
    orders: &Orders,
    order_status: &HashMap<String, bool>,
    src: &str,
) -> Bounds {
    let possibly_no_path = match is_path(map, state, orders, order_status, src) {
        Some(false) => return Bounds { min: 0, max: 0 },
        Some(true) => false,
        None => true,
    };

    let (dest_prov, dest_coast) = &orders[src]
        .downcast_ref::<Move>()
        .expect("src not a move order in compute_attack_strength")
        .dest;

    let mut possible_dest_nationality: Option<String> =
        state.units.get(dest_prov).map(|x| x.nationality());
    let dest_nationality_certain =
        if orders.contains_key(dest_prov) && orders[dest_prov].is::<Move>() {
            match order_status.get(dest_prov) {
                None => false,
                Some(true) => {
                    possible_dest_nationality = None;
                    true
                }
                Some(false) => true,
            }
        } else {
            true
        };

    let src_nationality = state
        .units
        .get(src)
        .map(Unit::nationality)
        .expect("empty unit in source of move order");
    if possible_dest_nationality == Some(src_nationality.clone()) && dest_nationality_certain {
        return Bounds { min: 0, max: 0 };
    }

    let mut bounds = Bounds { min: 1, max: 1 };
    for (prov_it, order_it) in orders.iter() {
        let support = match order_it.downcast_ref::<SupportMove>() {
            Some(s) => s,
            None => continue,
        };

        if !(support.src == src && support.dest == *dest_prov) {
            continue;
        }

        match order_status.get(prov_it) {
            Some(true) => {
                // if the destination nationality is definitely not the same as the supporter nationality, increase min + max by 1
                if possible_dest_nationality != Some(state.units[prov_it].nationality()) {
                    bounds.min += 1;
                    bounds.max += 1;
                }
                // if the destination nationality is possibly the same as the supporter nationality, increase max by 1
                else if possible_dest_nationality == Some(state.units[prov_it].nationality())
                    && !dest_nationality_certain
                {
                    bounds.max += 1;
                }
                // otherwise (if destination nationality is definitely the same as supporter nationality), support guaranteed to not count
            }
            Some(false) => {}
            None => {
                if !dest_nationality_certain
                    || possible_dest_nationality != Some(state.units[prov_it].nationality())
                {
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
pub fn compute_hold_strength(
    map: &Map,
    state: &MapState,
    orders: &Orders,
    order_status: &HashMap<String, bool>,
    target: &str,
) -> Bounds {
    if !state.units.contains_key(target) {
        return Bounds { max: 0, min: 0 };
    }

    if orders[target].is::<Move>() {
        // supports don't work
        match order_status.get(target) {
            Some(true) => return Bounds { max: 0, min: 0 },
            Some(false) => return Bounds { max: 1, min: 1 },
            None => return Bounds { max: 1, min: 0 },
        }
    }

    let mut bounds = Bounds { min: 1, max: 1 };
    for (prov_it, order_it) in orders.iter() {
        let support = match order_it.downcast_ref::<SupportHold>() {
            Some(s) => s,
            None => continue,
        };

        if support.target != target {
            continue;
        };

        match order_status.get(prov_it) {
            Some(true) => {
                bounds.min += 1;
                bounds.max += 1;
            }
            None => {
                bounds.min += 0;
                bounds.max += 1;
            }
            Some(false) => {
                bounds.min += 0;
                bounds.max += 0;
            }
        }
    }
    bounds
}

pub fn is_direct_path(map: &Map, state: &MapState, orders: &Orders, src: &str) -> bool {
    let (dest_prov, dest_coast) = &orders[src]
        .downcast_ref::<Move>()
        .expect("is_direct_path should have move order")
        .dest;

    match state
        .units
        .get(src)
        .expect("unit does not exist in is_path")
    {
        Unit::Army(_) => map
            .army_adj
            .contains(&(src.to_string(), dest_prov.to_string())),
        Unit::Fleet(_, src_coast) => {
            if !(map.provinces[dest_prov].coasts.contains(dest_coast)
                || (dest_coast == "" && map.provinces[dest_prov].coasts.is_empty()))
            {
                return false;
            }
            map.fleet_adj.contains(&(
                (src.to_string(), src_coast.to_string()),
                (dest_prov.to_string(), dest_coast.to_string()),
            ))
        }
    }
}

fn is_path_along(map: &Map, src: &str, dest: &str, convoys: &[String]) -> bool {
    let mut visited = HashSet::new();
    let mut stack = vec![src.to_string()];
//    println!("-- convoy {} --", src);
    loop {
        let node = match stack.pop() {
            Some(n) => n,
            None => return false,
        };
//        println!("node {}, {}",  &node, visited.len());

        if map.fleet_adj.contains(&(
            (node.to_string(), "".to_string()),
            (dest.to_string(), "".to_string()),
        )) && visited.len() != 0
        {
            return true;
        }

        for convoy in convoys.iter() {
            if visited.contains(convoy) || node == *convoy {
                continue;
            }
            if map.fleet_adj.contains(&(
                (node.to_string(), "".to_string()),
                (convoy.to_string(), "".to_string()),
            )) {
                stack.push(convoy.to_string());
            }
        }

        visited.insert(node);
    }
}

pub fn is_convoy_path(
    map: &Map,
    state: &MapState,
    orders: &Orders,
    order_status: &HashMap<String, bool>,
    src: &str,
) -> Option<bool> {
    let (dest_prov, _) = &orders[src]
        .downcast_ref::<Move>()
        .expect("is_convoy_path should have move order")
        .dest;

    match state
        .units
        .get(src)
        .expect("unit does not exist in is_path")
    {
        Unit::Fleet(_, _) => return Some(false),
        _ => {}
    }

    if map.provinces[dest_prov].is_sea {
        return Some(false);
    }

    let mut possible_convoys = Vec::new();
    let mut definite_convoys = Vec::new();
    for (prov_it, order_it) in orders.iter() {
        let convoy = match order_it.downcast_ref::<Convoy>() {
            Some(c) => c,
            None => continue,
        };

        if !(convoy.src == src && convoy.dest == *dest_prov) {
            continue;
        }

        match order_status.get(prov_it) {
            Some(true) => {
                possible_convoys.push(prov_it.to_string());
                definite_convoys.push(prov_it.to_string());
            }
            None => {
                possible_convoys.push(prov_it.to_string());
            }
            Some(false) => {}
        }
    }

    let is_definite = is_path_along(map, src, dest_prov, &definite_convoys);
    if is_definite {
        return Some(true);
    }
    let is_possible = is_path_along(map, src, dest_prov, &possible_convoys);
    if is_possible {
        return None;
    } else {
        return Some(false);
    }
}

pub fn is_path(
    map: &Map,
    state: &MapState,
    orders: &Orders,
    order_status: &HashMap<String, bool>,
    src: &str,
) -> Option<bool> {
    if is_direct_path(map, state, orders, src) {
        return Some(true);
    }

    is_convoy_path(map, state, orders, order_status, src)
}
