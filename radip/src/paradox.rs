
use std::{hash::Hash, thread::current};

use builtin::{compute_attack_strength, compute_defend_strength, compute_prevent_strength, is_path, Move};

use crate::*;

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