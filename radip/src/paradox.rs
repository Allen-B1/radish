
use std::{hash::Hash, thread::current};

use builtin::{compute_attack_strength, compute_defend_strength, compute_prevent_strength, is_path, Convoy, Move};
use frozenset::{Freeze, FrozenSet};

use crate::*;

/// Compute the cycle starting at the given province.
/// A cycle is a circular sequence of moves
/// for which each move has greater attack strength
/// than the prevent strength of all other units
/// moving to the same province.
fn get_cycle_at(map: &Map, state: &MapState, orders: &Orders, order_status: &mut HashMap<String, bool>, start: &str) -> Option<HashSet<String>> {
    let mut cycle = HashSet::new();

    let mut current_prov = start;
    loop {
        if !(orders.contains_key(current_prov) && orders[current_prov].is::<Move>() && order_status.get(current_prov).map(|x| *x) != Some(false) &&
            is_path(map, state, orders, order_status, current_prov) == Some(true)) {
            return None
        }

        cycle.insert(current_prov.to_string());

        let mov = orders[current_prov].downcast_ref::<Move>().unwrap();

        // if the attack might not succeed, invalid cycle
        let attack_strength = compute_defend_strength(map, state, orders, order_status, current_prov);
        let mut strengths = vec![];
        for (prov_it, order_it) in orders.iter() {
            if prov_it == current_prov {
                continue
            }
            if Move::is_move_to(order_it.deref(), &mov.dest.0) {
                strengths.push(compute_prevent_strength(map, state, orders, order_status, prov_it));
            }
        }

        let mut definitely_wins = true;
        for strength in strengths {
            if attack_strength.min <= strength.max {
                definitely_wins = false;
                break;
            }
        }

        if !definitely_wins {
            return None
        }

        // if destination is already in the cycle, invalid cycle
        if mov.dest.0 == start {
            break
        } else if cycle.contains(&mov.dest.0) {
            return None
        }

        current_prov = &mov.dest.0;
    }

    Some(cycle)
}

/// Resolve all move orders in
/// a cycle with success.
pub fn handle_cycles(map: &Map, state: &MapState, orders: &Orders, order_status: &mut HashMap<String, bool>) {
    loop {
        let num_resolved = order_status.len();

        for (prov_it, order_it) in orders {
            if order_status.get(prov_it) != None || !order_it.is::<Move>()  {
                continue
            }

            match get_cycle_at(map, state, orders, order_status, prov_it) {
                Some(cycle) => {
                    for item in cycle {
                        order_status.insert(item, true);
                    }
                    break
                }
                None => continue
            }
        }

        if num_resolved == order_status.len() {
            break
        }
    }
}

fn get_component(map: &Map, state: &MapState, orders: &Orders, order_status: &mut HashMap<String, bool>, start: &str) -> FrozenSet<String> {
    let mut component = HashSet::new();
    let mut visited = HashSet::new();
    let mut stack = vec![start.to_string()];

    loop {
        let node = match stack.pop() {
            Some(node) => node,
            None => break
        };

        if !orders.contains_key(&node) {
            continue
        }

        for dep in orders[&node].deps(map, state, orders, &node) {
            if !visited.contains(&dep) {
                stack.push(dep);
            }
        }

        if orders[&node].is::<Convoy>() {
            component.insert(node.to_string());
        }

        visited.insert(node);
    }
    
    component.freeze()
}

/// Compute the component of unresolved moves
/// with the minimum number of convoy orders,
/// and set all convoy orders in that component to fail.
/// 
/// If there are multiple components with the same
/// minimal number of convoy moves, all convoy orders
/// in all such components fail.
pub fn handle_convoy(map: &Map, state: &MapState, orders: &Orders, order_status: &mut HashMap<String, bool>) {
    let mut components: HashSet<FrozenSet<String>> = HashSet::new();
    for (prov_it, order_it) in orders.iter() {
        if order_it.is::<Convoy>() && order_status.get(prov_it) == None {
            components.insert(get_component(map, state, orders, order_status, prov_it));
        }
    }

    let min = match components.iter().map(|x| x.len()).min() {
        None => return,
        Some(min) => min
    };

    for component in components {
        if component.len() == min {
            for convoy_prov in component {
                order_status.insert(convoy_prov, false);
            }
        }
    }
}