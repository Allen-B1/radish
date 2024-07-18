use std::{collections::{HashSet, HashMap}, hash::Hash};

pub type FleetLoc = (String, String);
pub type ArmyLoc = String;

#[derive(Clone,Copy)]
pub enum UnitType {
    Fleet,
    Army
}

#[derive(Clone)]
pub struct Province {
    pub full_name: String,
    pub coasts: HashSet<String>,
}

#[derive(Clone)]
pub struct Map {
    pub provinces: HashMap<String, Province>,
    pub fleet_adj: HashSet<(FleetLoc, FleetLoc)>,
    pub army_adj: HashSet<(ArmyLoc, ArmyLoc)>
}

#[derive(Clone, Debug)]
pub struct MapState {
    pub armies: HashMap<ArmyLoc, String>,
    pub fleets: HashMap<FleetLoc, String>,
}

#[derive(Clone, PartialEq)]
pub enum Order {
    Hold,
    Move(FleetLoc),
    SupportMove(String, String),
    SupportHold(String),
    Convoy(String, String),
}

impl Order {
    pub fn is_move_to(&self, province: &str) -> bool {
        match self {
            Order::Move((prov, coast)) => prov == province,
            _ => false
        }
    }
    pub fn is_move(&self) -> bool {
        match self {
            Order::Move(_) => true,
            _ => false
        }
    }
}

struct Bounds {
    pub min: u32,
    pub max: u32
}

pub fn adjudicate(map: Map, state: MapState, orders: HashMap<String, Order>) -> HashMap<String, bool> {
    // not in hashmap = unknown
    let mut order_status: HashMap<String, bool> = HashMap::new();
    loop {
        // TODO: compute cycles

        let resolved_order_count = order_status.len();
        for (province, order) in orders.iter() {
            if order_status.contains_key(province) {
                continue
            }
            match order {
                Order::SupportHold(_) | Order::SupportMove(_, _) => {
                    match order {
                        Order::SupportHold(dest) => {
                            if orders.get(dest).unwrap_or(&Order::Hold).is_move() {
                                order_status.insert(province.to_string(), false);
                                continue;
                            }
                        } 
                        _ => {}
                    }

                    let mut no_successful_path = true;
                    for (province2, order2) in orders.iter() {
                        if order2.is_move_to(province) {
                            match is_path(map, state, orders, order_status, province2) {
                                Some(true) => {
                                    order_status.insert(province.to_string(), false);
                                    no_successful_path = false;
                                    break;
                                },
                                None => {
                                    no_successful_path = false;
                                    break;
                                },
                                Some(false) => continue
                            }
                        }
                    }
                    if no_successful_path {
                        order_status.insert(province.to_string(), true);
                    }
                },
                Order::Convoy(_, _) | Order::Hold => {
                    let mut no_successful_move = true;
                    for (province2, order2) in orders.iter() {
                        if order2.is_move_to(province) {
                            match order_status.get(province2.as_str()) {
                                Some(&true) => {
                                    order_status.insert(province.to_string(), false);
                                    no_successful_move = false;
                                    break;
                                },
                                None => {
                                    no_successful_move = false;
                                    break;
                                },
                                Some(false) => continue
                            }
                        }
                    }
                    if no_successful_move {
                        order_status.insert(province.to_string(), true);
                    }
                },
                Order::Move((dest_prov, dest_coast)) => {
                    let attack_strength = compute_attack_strength(map, state, orders, order_status, province.as_str());

                    let strengths: Vec<Bounds> = Vec::new();
                    if orders.get(dest_prov).unwrap_or(&Order::Hold).is_move_to(dest_prov) {
                        // head-to-head battle
                        strengths.push(compute_defend_strength(map, state, orders, order_status, dest_prov.as_str()));
                    } else {
                        strengths.push(compute_hold_strength(map, state, orders, order_status, dest_prov.as_str()));
                        // not head-to-head battle
                    }
                    for (province2, order2) in orders.iter() {
                        if order2.is_move_to(dest_prov) {
                            strengths.push(compute_prevent_strength(map, state, orders, order_status, province2.as_str()));
                        }
                    }

                    let mut all_successful = true;
                    for strength in strengths.iter() {
                        if attack_strength.max <= strength.min {
                            order_status.insert(province.to_string(), false);
                            all_successful = false;
                            break;
                        }
                        if attack_strength.min <= strength.max {
                            all_successful = false;
                        }    
                    }
                    if all_successful {
                        order_status.insert(province.to_string(), true);
                    }
                }
            }
        }

        if resolved_order_count == order_status.len() {
            // no orders were resolved on this step
            break;
        }
    }

    order_status
}