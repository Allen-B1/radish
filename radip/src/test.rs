use std::{
    collections::{HashMap, HashSet},
    error::Error,
    hash::Hash,
};

use crate::{adjudicate, builtin, Map, MapState, Orders, Unit};

#[derive(Debug)]
struct Test {
    pub name: String,
    pub units: HashMap<String, Unit>,
    pub orders: Orders,
    pub expected: HashMap<String, bool>,
}

impl Test {
    pub fn empty(name: String) -> Self {
        Test {
            name,
            units: HashMap::new(),
            orders: HashMap::new(),
            expected: HashMap::new(),
        }
    }
}

fn parse_province(src: &str) -> (String, String) {
    let lparen = src.find("(");
    let rparen = src.find(")");
    match (lparen, rparen) {
        (None, _) | (_, None) => (src.to_lowercase(), "".to_string()),
        (Some(i), Some(j)) => (src[0..i].to_lowercase(), src[i + 1..j].to_lowercase()),
    }
}

#[test]
fn datc() -> Result<(), Box<dyn Error>> {
    let stdvar = include_str!("../data/std.json");
    let map: Map = serde_json::from_str(stdvar)?;
    let datc = include_str!("../data/datc.md");

    let mut tests = vec![];
    let mut active_test = Test::empty("".to_string());
    let mut active_nation = "".to_string();
    for line in datc.lines() {
        let line = line.trim();
        if line.starts_with("###") {
            println!("{}", line);
            if active_test.name != "" {
                tests.push(active_test);
            }
            active_test = Test::empty(line[3..].trim().to_string());
            continue;
        }
        if line.ends_with(":") {
            active_nation = line[..line.len() - 1].trim().to_string();
            continue;
        }
        if line.starts_with("`") {
            let end = line[1..].find("`");
            if end == None {
                continue;
            }

            let order_str = &line[1..end.unwrap() + 1];

            let parts = order_str.split(" ").collect::<Vec<_>>();
            let (prov, coast) = parse_province(parts[1]);
            match parts[0] {
                "F" => {
                    active_test.units.insert(
                        prov.to_string(),
                        Unit::Fleet(active_nation.to_string(), coast.to_string()),
                    );
                }
                "A" => {
                    active_test
                        .units
                        .insert(prov.to_string(), Unit::Army(active_nation.to_string()));
                }
                x => {
                    panic!("unknown unit type {}", x)
                }
            }

            match parts[2] {
                "-" => {
                    let (dest_prov, dest_coast) = parse_province(parts[3]);
                    active_test.orders.insert(
                        prov.to_string(),
                        Box::new(builtin::Move {
                            dest: (dest_prov.to_string(), dest_coast.to_string()),
                        }),
                    );
                }
                "S" => {
                    if parts[4] == "-" {
                        let (src_prov, src_coast) = parse_province(parts[3]);
                        let (dest_prov, dest_coast) = parse_province(parts[5]);
                        active_test.orders.insert(
                            prov.to_string(),
                            Box::new(builtin::SupportMove {
                                src: src_prov.to_string(),
                                dest: dest_prov.to_string(),
                            }),
                        );
                    } else if parts[4] == "H" {
                        let (dest_prov, dest_coast) = parse_province(parts[3]);
                        active_test.orders.insert(
                            prov.to_string(),
                            Box::new(builtin::SupportHold {
                                target: dest_prov.to_string(),
                            }),
                        );
                    } else {
                        panic!("unknown support order: {}", parts[4]);
                    }
                }
                "H" => {
                    active_test
                        .orders
                        .insert(prov.to_string(), Box::new(builtin::Hold));
                }
                "C" => {
                    assert!(parts[4] == "-");
                    let (src_prov, src_coast) = parse_province(parts[3]);
                    let (dest_prov, dest_coast) = parse_province(parts[5]);
                    active_test.orders.insert(
                        prov.to_string(),
                        Box::new(builtin::Convoy {
                            src: src_prov.to_string(),
                            dest: dest_prov.to_string(),
                        }),
                    );
                }
                _ => {
                    panic!("unknown order")
                }
            }

            match line[end.unwrap() + 2..].trim() {
                "T" => {
                    active_test.expected.insert(prov.to_string(), true);
                }
                "F" => {
                    active_test.expected.insert(prov.to_string(), false);
                }
                x => {
                    panic!("undefined test result: {}", x)
                }
            }
        }
    }

    tests.push(active_test);

    let mut failed = Vec::new();
    let mut incomplete = Vec::new();
    let tests_len = tests.len();
    for test in tests {
        let mut failed_test = false;
        let mut incomplete_test = false;

        println!("\x1b[0m\x1b[1mtest {}\x1b[0m", test.name);
        let state = MapState { units: test.units };
        let results = adjudicate(&map, &state, &test.orders);
        for (prov, expected) in test.expected {
            if results.get(&prov).map(|x| *x) == Some(expected) {
                print!("\x1b[32m")
            } else if results.get(&prov) == None {
                incomplete_test = true;
                print!("\x1b[33m")
            } else {
                failed_test = true;
                print!("\x1b[31m")
            }

            println!(
                "{}: {:?} | {}",
                &prov,
                &test.orders[&prov],
                match results.get(&prov) {
                    Some(true) => "T",
                    Some(false) => "F",
                    None => "X",
                }
            );
        }

        if failed_test {
            failed.push(test.name);
        } else if incomplete_test {
            incomplete.push(test.name);
        }
    }

    println!("");
    println!(
        "\x1b[31mfailed \x1b[0m[{}/{}]: {}",
        failed.len(),
        tests_len,
        failed.join(", ")
    );
    println!(
        "\x1b[33mincomplete \x1b[0m[{}/{}]: {}",
        incomplete.len(),
        tests_len,
        incomplete.join(", ")
    );

    Ok(())
}
