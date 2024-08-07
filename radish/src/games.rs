use std::{collections::{HashMap, HashSet}, future::pending, hash::Hash, io::{Cursor, Read}, time::{SystemTime, UNIX_EPOCH}};
use rand::prelude::*;
use radip::{adjudicate, base::{Hold, Move}, utils::{apply_adjudication, MapMeta, RetreatOptions}, Map, MapState, Orders, ProvinceAbbr, Unit};
use rocket::{build, form::Form, fs::{NamedFile, TempFile}, futures::{SinkExt, StreamExt}, http::{CookieJar, Status}, response::{content::RawHtml, Redirect}, serde::{json::Json, Deserialize, Serialize}, tokio::{io::AsyncReadExt, select, sync::broadcast, time::{Duration, Instant}}, State};
use tokio::{sync::broadcast::error::RecvError, time};
use ws::{stream::DuplexStream, Message};

use crate::{encode_error, gen_id, AppState, HeadComponent, HeaderComponent, Variant};

#[litem::template("pages/create_game.html")]
struct CreateGamePage {
    user_name: String,
}

#[get("/games/new")]
pub fn create_game_page(cookies: &CookieJar<'_>, state: &State<AppState>) -> Result<RawHtml<String>, Redirect> {
    let token = cookies.get("token").map(|c| c.value()).unwrap_or("");
    let user = state.users.get(token).ok_or(Redirect::to("/signin"))?;

    Ok(RawHtml(CreateGamePage {
        user_name: user.name.clone()
    }.render_string().unwrap()))
}

#[post("/games/new/submit", data = "<form>")]
pub async fn create_game_submit(cookies: &CookieJar<'_>, state: &State<AppState>, form: Form<CreateGameForm<'_>>) -> Result<Redirect, Redirect> {
    let form = form.into_inner();
    let token = cookies.get("token").map(|c| c.value()).unwrap_or("");
    let user = state.users.get(token).ok_or(Redirect::to("/signin"))?;

    let mut buf: Vec<u8> = vec![];
    form.variant.open().await
        .map_err(|e| Redirect::to(format!("/error?details={}", encode_error(e))))?
        .read_to_end(&mut buf).await
        .map_err(|e| Redirect::to(format!("/error?details={}", encode_error(e))))?;

    let mut zip  = zip::ZipArchive::new(Cursor::new(buf))
        .map_err(|e| Redirect::to(format!("/error?details={}", encode_error(e))))?;

    let meta: MapMeta = rmp_serde::from_read(zip.by_name("meta.mpk")
        .map_err(|e| Redirect::to(format!("/error?msg=Invalid+variant+file&details={}", encode_error(e))))?)
        .map_err(|e| Redirect::to(format!("/error?msg=Invalid+variant+file&details={}", encode_error(e))))?;

    let variant_id = meta.data.get("id").and_then(|v| v.as_str())
        .ok_or(Redirect::to(format!("/error?msg=Invalid+variant+file&details={}", encode_error("missing id".to_string()))))?;

    if !state.variants.contains_key(variant_id) {
        let mut map = String::new();
        zip.by_name("map.svg")
            .map_err(|e| Redirect::to(format!("/error?msg=Invalid+variant+file&details={}", e)))?
            .read_to_string(&mut map)
            .map_err(|e| Redirect::to(format!("/error?msg=Invalid+variant+file&details={}", e)))?;

        let adj: Map = rmp_serde::from_read(zip.by_name("adj.mpk")
            .map_err(|e| Redirect::to(format!("/error?msg=Invalid+variant+file&details={}", encode_error(e))))?)
            .map_err(|e| Redirect::to(format!("/error?msg=Invalid+variant+file&details={}", encode_error(e))))?;

        let pos: PosData =  rmp_serde::from_read(zip.by_name("pos.mpk")
        .map_err(|e| Redirect::to(format!("/error?msg=Invalid+variant+file&details={}", encode_error(e))))?)
        .map_err(|e| Redirect::to(format!("/error?msg=Invalid+variant+file&details={}", encode_error(e))))?;

        state.variants.insert(variant_id.to_string(), Variant {
            adj: adj,
            svg: map,
            meta: meta.clone(),
            pos: pos,
        });
    }

    let (sender, _) = broadcast::channel(16);

    let game_id = gen_id();
    state.games.insert(game_id.clone(), Game {
        meta: GameMeta {
            name: form.name,
            press: form.press,
            end_year: form.end_year,

            variant: variant_id.to_string(),

            time_build: match form.time_build_unit {
                TimeUnit::Min => Duration::from_secs(60*form.time_build as u64),
                TimeUnit::Hr => Duration::from_secs(60*60*form.time_build as u64)
            },
            time_mvmt: match form.time_mvmt_unit {
                TimeUnit::Min => Duration::from_secs(60*form.time_mvmt  as u64),
                TimeUnit::Hr => Duration::from_secs(60*60*form.time_mvmt  as u64)
            }
        },
        broadcast: sender,
        player_broadcast: HashMap::new(),
        state: None
    });

    Ok(Redirect::to(format!("/games/{}", game_id)))
}

#[derive(FromFormField, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum PressType {
    Full,
    Rulebook,
    Public,
    Gunboat
}

#[derive(FromFormField)]
pub enum TimeUnit {
    Hr,
    Min
}

#[derive(FromForm)]
struct CreateGameForm<'a> {
    pub name: String,
    pub press: PressType,
    pub end_year: Option<u8>,

    pub time_mvmt: u8,
    pub time_mvmt_unit: TimeUnit,

    pub time_build: u8,
    pub time_build_unit: TimeUnit,

    pub variant: TempFile<'a>
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PosEntry {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PosData {
    pub provinces: HashMap<ProvinceAbbr, PosEntry>,
    pub width: u32,
    pub height: u32
}

#[derive(Clone, Serialize, Deserialize)]
pub struct GameMeta {
    pub name: String,
    pub press: PressType,
    pub end_year: Option<u8>,

    pub time_mvmt: Duration,
    pub time_build: Duration,

    pub variant: String,
}

#[derive(Clone, Serialize, Debug, Deserialize, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all="snake_case")]
pub enum GamePhase {
    Spring,
    SpringRetreat,
    Fall,
    FallRetreat,
    Winter
}
impl GamePhase {
    pub fn is_move(self) -> bool {
        match self {
            GamePhase::Fall | GamePhase::Spring => true,
            GamePhase::SpringRetreat | GamePhase::FallRetreat => false,
            GamePhase::Winter => false,
        }
    }
    pub fn is_retreat(self) -> bool {
        match self {
            GamePhase::Fall | GamePhase::Spring => false,
            GamePhase::SpringRetreat | GamePhase::FallRetreat => true,
            GamePhase::Winter => false,
        }
    }
    pub fn is_build(self) -> bool {
        match self {
            GamePhase::Fall | GamePhase::Spring => false,
            GamePhase::SpringRetreat | GamePhase::FallRetreat => false,
            GamePhase::Winter => true,
        }
    }

    pub fn next(self, year: u8) -> (Self, u8) {
        match self {
            GamePhase::Spring => (GamePhase::SpringRetreat, year),
            GamePhase::SpringRetreat => (GamePhase::Fall, year),
            GamePhase::Fall => (GamePhase::FallRetreat, year),
            GamePhase::FallRetreat => (GamePhase::Winter, year),
            GamePhase::Winter => (GamePhase::Spring, year+1)
        }
    }
    pub fn mvmt(self) -> Self {
        match self {
            GamePhase::FallRetreat | GamePhase::Fall => GamePhase::Fall,
            GamePhase::SpringRetreat | GamePhase::Spring => GamePhase::Spring,
            GamePhase::Winter => GamePhase::Winter
        }
    }
}

#[derive(Clone)]
pub struct MvmtPhaseInfo {
    order_status: HashMap<String, bool>,
    retreats: HashMap<String, RetreatOptions>
}

type Builds = HashMap<String, Unit>;

pub struct GameState {
    /// For each phase, this exists on phase start.
    pub states: HashMap<(u8, GamePhase), MapState>,
    /// For each movement or retreat phase, this exists on phase start.
    pub orders: HashMap<(u8, GamePhase), Orders>,
    /// For each movement phase, this exists after the retreat phase starts.
    pub mvmt_info: HashMap<(u8, GamePhase), MvmtPhaseInfo>,
    /// For each build phase, this might or might not exist.
    pub builds: HashMap<(u8, GamePhase), Builds>,

    pub year: u8,
    pub phase: GamePhase,
    pub adj_time: tokio::time::Instant,
    
    // user id => power
    pub players: HashMap<String, String>,
}

impl GameState {
    pub fn current_state(&self) -> &MapState {
        &self.states[&(self.year, self.phase)]
    }
    pub fn current_state_mut(&mut self) -> &mut MapState {
        self.states.get_mut(&(self.year, self.phase)).unwrap()
    }
    pub fn current_orders(&self) -> &Orders {
        &self.orders[&(self.year, self.phase)]
    }
    pub fn current_orders_mut(&mut self) -> &mut Orders {
        self.orders.get_mut(&(self.year, self.phase)).unwrap()
    }
}

#[derive(Deserialize, Clone)]
#[serde(rename_all="snake_case", tag = "type")]
pub enum InMessage {
    Auth { token: String },
    Orders { orders: Orders },
    Builds { builds: Builds }
}

#[derive(Serialize, Clone)]
#[serde(rename_all="snake_case", tag = "type")]
pub enum OutMessage {
    Error { msg : String },
    UpdatePlayers { players: Vec<(String, String)> },

    GameInfo {
        power: String,
    },

    MapState {
        year: u8,
        phase: GamePhase,
        state: MapState
    },

    Phase {
        year: u8,
        phase: GamePhase,
        adj_time: u64,
        state: MapState,
    },

    MovementAdj {
        year: u8,
        phase: GamePhase,
        orders: Orders,
        order_status: HashMap<String, bool>,
        retreats: HashMap<String, RetreatOptions>,
    },

    RetreatAdj {
        year: u8,
        phase: GamePhase,
        orders: Orders
    },

    BuildAdj {
        year: u8,
        phase: GamePhase,
        builds: HashMap<String, Unit>,
    }
}

pub struct Game {
    pub meta: GameMeta,
    pub broadcast: broadcast::Sender<OutMessage>,
    pub player_broadcast: HashMap<String, broadcast::Sender<OutMessage>>,
    pub state: Option<GameState>
}


#[get("/games/<id>")]
pub async fn game(id: &str) -> Option<NamedFile> {
    NamedFile::open(rocket::fs::relative!("pages/game/dist").to_string() + "/index.html").await.ok()
}

#[get("/games/<id>/meta.json")]
pub fn game_meta(state: &State<AppState>, id: &str) -> Result<Json<GameMeta>, Status> {
    let game: dashmap::mapref::one::Ref<String, Game> = state.games.get(id).ok_or(Status::NotFound)?;
    Ok(Json(game.meta.clone()))
}

/// While this thread runs,
/// the game at `game_id` should exist.
async fn game_thread(state: AppState, game_id: String) {
    // initialize game
    // - decide powers
    // - set state to Some
    // - send broadcast messages
    {
        let mut game = state.games.get_mut(&game_id).unwrap();
        let variant = state.variants.get(&game.meta.variant).unwrap();
        let adj_time = Instant::now() + game.meta.time_mvmt;
        let mut players = HashMap::new();
        let mut powers = variant.meta.powers.keys().map(|x| x.clone()).collect::<Vec<String>>();
        powers.shuffle(&mut thread_rng());

        for player in game.player_broadcast.keys() {
            players.insert(player.clone(), powers.pop().unwrap());
        }

        game.broadcast.send(OutMessage::UpdatePlayers {
            players: players.iter().map(|(t,pwr)| (pwr.to_string(), state.users.get(t).map(|c| c.name.to_string()).unwrap_or("".to_string()))).collect() 
        });

        for (player, power) in players.iter() {
            game.player_broadcast[player].send(OutMessage::GameInfo { 
                power: power.to_string()
            });
        }

        game.state = Some(GameState {
            year: 1,
            phase: GamePhase::Spring,
            adj_time: adj_time,

            states: HashMap::from([((1, GamePhase::Spring), variant.meta.starting_state.clone())]),
            orders: HashMap::from([((1, GamePhase::Spring), HashMap::new())]),
            mvmt_info: HashMap::new(),
            builds: HashMap::new(),

            players
        });

        drop(game);
        drop(variant);
    }

    loop {
        let mut game = state.games.get_mut(&game_id).unwrap();

        let gstate = game.state.as_ref().unwrap();
        let new_adj_time;

        // skip empty retreat phases
        if gstate.phase.is_retreat() && gstate.mvmt_info.get(&(gstate.year, gstate.phase.mvmt())).map(|i| i.retreats.len()).unwrap_or(0) == 0 {
            new_adj_time = Instant::now();
        } else {
            new_adj_time = Instant::now() + match gstate.phase.is_move() {
                true => game.meta.time_mvmt,
                false => game.meta.time_build
            };
        }

        let gstate = game.state.as_mut().unwrap();
        gstate.adj_time = new_adj_time;
        
        let gstate = game.state.as_ref().unwrap();
        let epoch = Instant::now() - (SystemTime::now().duration_since(std::time::UNIX_EPOCH)).expect("unable to get system time");
        game.broadcast.send(OutMessage::Phase {
            year: gstate.year, phase: gstate.phase,
            adj_time: gstate.adj_time.duration_since(epoch).as_millis() as u64,
            state: gstate.current_state().clone()
        });

        let adj_time = gstate.adj_time;
        drop(game);
        time::sleep_until(adj_time).await;
 
        let mut game = state.games.get_mut(&game_id).unwrap();
        let variant = state.variants.get(&game.meta.variant).unwrap();
        let gstate = game.state.as_ref().unwrap();
        println!("adjudicating {:?} {}", gstate.phase, gstate.year);

        // sanitize orders
        if gstate.phase.is_move() {
            let gstate = game.state.as_mut().unwrap();
            let occupied_tiles = gstate.current_state().units.keys().map(|s| s.to_string()).collect::<Vec<_>>();
            let orders = gstate.current_orders_mut();
            for prov in occupied_tiles {
                orders.entry(prov).or_insert(Box::new(Hold));
            }
            drop(orders);
            drop(gstate);
        }
         
        let gstate = game.state.as_ref().unwrap();
        if gstate.phase.is_move() {
            let orders = gstate.current_orders();
            let order_status = adjudicate(&variant.adj, gstate.current_state(), orders);
            let (new_mstate, retreats) = apply_adjudication(&variant.adj, gstate.current_state(), orders, &order_status);

            game.broadcast.send(OutMessage::MovementAdj { 
                year: gstate.year,
                phase: gstate.phase,
                orders: gstate.current_orders().clone(),
                order_status: order_status.clone(),
                retreats: retreats.clone()
            });

            // mutate
            let gstate = game.state.as_mut().unwrap();
            gstate.mvmt_info.insert((gstate.year, gstate.phase), MvmtPhaseInfo {
                retreats,
                order_status
            });

            (gstate.phase, gstate.year) = gstate.phase.next(gstate.year);
            gstate.states.insert((gstate.year, gstate.phase), new_mstate);
            gstate.orders.insert((gstate.year, gstate.phase), HashMap::new());
        } else if gstate.phase.is_retreat() {
            let mut new_mstate = gstate.current_state().clone();
            let mvmt_info = gstate.mvmt_info.get(&(gstate.year, gstate.phase.mvmt())).unwrap();

            let mut retreat_units: HashMap<String, Vec<Unit>> = HashMap::new();

            for (prov, order) in gstate.orders.get(&(gstate.year, gstate.phase)).unwrap().iter() {
                let mov = match order.downcast_ref::<Move>() {
                    None => continue,
                    Some(mov) => mov
                };
                let retreats = match mvmt_info.retreats.get(prov) {
                    None => continue,
                    Some(r) => r
                };
                if !retreats.dest.contains(&(mov.dest.0.clone(), mov.dest.1.clone())) {
                    continue;
                }

                let vec = retreat_units.entry(mov.dest.0.to_string()).or_default();
                vec.push(match &retreats.src {
                    Unit::Army(natl) => Unit::Army(natl.clone()),
                    Unit::Fleet(natl, _) => Unit::Fleet(natl.clone(), mov.dest.1.clone())
                });
            }

            for (prov, units) in retreat_units {
                if units.len() == 1 {
                    new_mstate.units.insert(prov, units.into_iter().next().unwrap());
                }
            }

            game.broadcast.send(OutMessage::RetreatAdj { 
                year: gstate.year,
                phase: gstate.phase,
                orders: gstate.current_orders().clone(),
            });

            // mutate
            let gstate = game.state.as_mut().unwrap();
            (gstate.phase, gstate.year) = gstate.phase.next(gstate.year);
            gstate.states.insert((gstate.year, gstate.phase), new_mstate);
            gstate.orders.insert((gstate.year, gstate.phase), HashMap::new());

            // update ownership
            if gstate.phase.is_build() {
                let mstate = gstate.current_state_mut();
                for (prov, unit) in mstate.units.iter() {
                    if variant.meta.provinces.get(prov).map(|p| p.is_sc).unwrap_or(false) {
                        mstate.ownership.insert(prov.to_string(), unit.nationality());
                    }
                }
            }
        } else if gstate.phase.is_build() {
            let mut new_mstate = gstate.current_state().clone();
            let builds  =HashMap::new();
            let builds = gstate.builds.get(&(gstate.year, gstate.phase.mvmt())).unwrap_or(&builds);
            for (prov, build) in builds {
                new_mstate.units.insert(prov.to_string(), build.clone());
            }

            game.broadcast.send(OutMessage::BuildAdj { 
                year: gstate.year,
                phase: gstate.phase,
                builds: builds.clone()
            });

            let gstate = game.state.as_mut().unwrap();
            (gstate.phase, gstate.year) = gstate.phase.next(gstate.year);
            gstate.states.insert((gstate.year, gstate.phase), new_mstate);
            gstate.orders.insert((gstate.year, gstate.phase), HashMap::new());
        }

        drop(game);
        drop(variant);
    }
}

async fn handle_in_message(state: &AppState, game_id: &str, token: &mut String, player_broadcast: &mut Option<broadcast::Receiver<OutMessage>>, msg: InMessage, stream: &mut DuplexStream) -> Result<(), ()> {
    async fn send(stream: &mut DuplexStream, msg: OutMessage) {
        stream.send(Message::Text(serde_json::to_string(&msg).unwrap())).await;
    };

    match msg {
        InMessage::Auth { token: tok } => {
            println!("player joined");

            let game = state.games.get(game_id).ok_or(())?;   
            if state.users.contains_key(&tok) && game.state.is_none() {
                drop(game);
                let mut game = state.games.get_mut(game_id).ok_or(())?;   
                if !game.player_broadcast.contains_key(&tok) {
                    let (sender, _) = broadcast::channel(16);
                    game.player_broadcast.insert(tok.clone(), sender);

                    let update_players_msg = OutMessage::UpdatePlayers { players: game.player_broadcast.keys().map(|id| state.users.get(id).map(|p| p.name.clone()).unwrap_or("".to_string())).map(|n| ("".to_string(), n.to_string())).collect() };
                    _ = game.broadcast.send(update_players_msg.clone());
                    send(stream, update_players_msg).await;  

                    let variant = state.variants.get(&game.meta.variant).unwrap();
                    if game.player_broadcast.len() == variant.meta.powers.len() {
                        // start game
                        let state_clone = state.clone();
                        let game_id_clone = game_id.to_string();
                        tokio::spawn(async move {
                            game_thread(state_clone, game_id_clone).await
                        });
                    }
                }

                *player_broadcast = Some(game.player_broadcast[&tok].subscribe());
            }
            *token = tok;

            // send starting information
            if let Some(game) = state.games.get(game_id) {
                println!("sending starting info...");
                if let Some(gstate) = &game.state {
                    for (&(year, phase), state) in gstate.states.iter() {
                        send(stream, OutMessage::MapState {
                            year, phase, state: state.clone()
                        }).await;
                    }
                    for (&(year, phase), orders) in gstate.orders.iter() {
                        if (year, phase) == (gstate.year, gstate.phase) {
                            continue
                        }
                        if phase.is_move() {
                            send(stream, OutMessage::MovementAdj {
                                year, phase,
                                orders: orders.clone(),
                                order_status: gstate.mvmt_info[&(year, phase)].order_status.clone(),
                                retreats: gstate.mvmt_info[&(year, phase)].retreats.clone()
                            }).await;
                        } else if phase.is_retreat() {
                            send(stream, OutMessage::RetreatAdj {
                                year, phase,
                                orders: orders.clone()
                            }).await;
                        }
                    }
                    for (&(year, phase), builds) in gstate.builds.iter() {
                        if (year, phase) == (gstate.year, gstate.phase) {
                            continue
                        }
                        send(stream, OutMessage::BuildAdj { year, phase, builds: builds.clone() }).await;
                    }

                    let epoch = Instant::now() - (SystemTime::now().duration_since(std::time::UNIX_EPOCH)).expect("unable to get system time");

                    send(stream, OutMessage::GameInfo { power: gstate.players.get(token).map(|s| s.as_str()).unwrap_or("").to_string() }).await;
                    send(stream, OutMessage::Phase {
                        phase: gstate.phase, year: gstate.year,
                        adj_time: gstate.adj_time.duration_since(epoch).as_millis() as u64, state: gstate.current_state().clone()
                    }).await;
                    send(stream, OutMessage::UpdatePlayers {
                        players: gstate.players.iter().map(|(tok, pwr)| (pwr.to_string(), state.users.get(tok).unwrap().name.to_string())).collect()
                    }).await;
                }

                println!("done sending starting info");

                drop(game);
            }
        },
        InMessage::Builds { builds } => {
            let mut game = state.games.get_mut(game_id).ok_or(())?;   
            if game.state.is_none() {
                send(stream, OutMessage::Error { msg: "Game not started".to_string() }).await;
                return Ok(())
            }
            if !state.users.contains_key(token) || !game.player_broadcast.contains_key(token) {
                send(stream, OutMessage::Error { msg: "Not authenticated".to_string() }).await;
                return Ok(())
            }

            let gstate = game.state.as_mut().unwrap();
            if !gstate.phase.is_build() {
                send(stream, OutMessage::Error { msg: "Not a build phase".to_string() }).await;
                return Ok(())
            }

            let power = &gstate.players[&*token];
            for (prov, unit) in builds {
                if gstate.current_state().units.contains_key(&prov) {
                    send(stream, OutMessage::Error { msg: format!("Build location {} is occupied", &prov) }).await;
                    return Ok(())           
                }
                if gstate.current_state().ownership.get(&prov) != Some(&unit.nationality()) || unit.nationality() != *power {
                    send(stream, OutMessage::Error { msg: format!("Build location {} is not owned by you", &prov) }).await;
                    return Ok(())
                }

                let builds = gstate.builds.entry((gstate.year, gstate.phase)).or_insert(HashMap::new());
                builds.insert(prov, unit);
            }
        },
        InMessage::Orders { orders } => {
            let mut game = state.games.get_mut(game_id).ok_or(())?;   
            if game.state.is_none() {
                send(stream, OutMessage::Error { msg: "Game not started".to_string() }).await;
                return Ok(())
            }
            if !state.users.contains_key(token) || !game.player_broadcast.contains_key(token) {
                send(stream, OutMessage::Error { msg: "Not authenticated".to_string() }).await;
                return Ok(())
            }

            let gstate = game.state.as_mut().unwrap();
            if gstate.phase.is_build() {
                send(stream, OutMessage::Error { msg: "Not a movement or retreat phase".to_string() }).await;
                return Ok(())
            }

            if !gstate.players.contains_key(token) {
                send(stream, OutMessage::Error { msg: "You are not in this game".to_string() }).await;
                return Ok(())
            }

            // authenticate 
            if gstate.phase.is_move() {
                let power = gstate.players[token].as_str();
                for (prov, order) in orders.iter() {
                    if gstate.current_state().units.get(prov).map(|u| u.nationality()).unwrap_or("".to_string()) != power {       
                        send(stream, OutMessage::Error { msg: format!("Invalid orderset: you do not have a unit at {}", prov) }).await;
                        return Ok(())
                    }
                }
            } else {
                let power = gstate.players[token].as_str();
                for (prov, order) in orders.iter() {
                    let mov = match order.downcast_ref::<Move>() {
                        Some(mov) => mov,
                        None => {
                            send(stream, OutMessage::Error { msg: format!("Invalid orderset: only move orders allowed during adjudication") }).await;
                            return Ok(())    
                        }
                    };

                    let info = match  gstate.mvmt_info.get(&(gstate.year, gstate.phase.mvmt())) {
                        Some(info) => info,
                        None => {
                            send(stream, OutMessage::Error { msg: format!("Movement info not found") }).await;
                            return Ok(())    
                        }
                    };

                    if info.retreats.get(prov).is_none() || !info.retreats[prov].dest.contains(&mov.dest) {
                        send(stream, OutMessage::Error { msg: format!("Cannot retreat {} to {}", prov, &mov.dest.0) }).await;
                        return Ok(())    
                    }

                    if info.retreats[prov].src.nationality() != power {
                        send(stream, OutMessage::Error { msg: format!("{} is {}, you are {}", prov, info.retreats[prov].src.nationality(), power) }).await;
                        return Ok(())    
                    }
                }
            }

            gstate.current_orders_mut().extend(orders);
        }
    }

    Ok(())
}

#[get("/games/<id>/ws")]
pub fn game_stream(state: &State<AppState>, id: &str, ws: ws::WebSocket) -> Result<ws::Channel<'static>, Status> {
    let game = state.games.get(id).ok_or(Status::NotFound)?;
    let mut broadcast = game.broadcast.subscribe();
    drop(game);

    let id = id.to_string();
    let state = state.inner().clone();

    Ok(ws.channel(move |mut stream| Box::pin(async move {
        let mut token = String::new();
        let mut player_broadcast: Option<broadcast::Receiver<OutMessage>> = None;

        loop {
            select! {
                message = stream.next() => {
                    if message.is_none() {
                        break
                    }
                    match message.unwrap()? {
                        Message::Text(text) => {
                            let msg: InMessage = match serde_json::from_str(&text) {
                                Ok(msg) => msg,
                                Err(err) => { eprintln!("{:?}", err); continue }
                            };
                            
                            let _ = handle_in_message(&state, &id, &mut token, &mut player_broadcast, msg, &mut stream).await;
                        },
                        Message::Ping(data) => {
                            stream.send(Message::Pong(data)).await?;
                        },
                        _ => {}
                    }
                },

                Ok(message) = broadcast.recv() => {
                    stream.send(Message::Text(serde_json::to_string(&message).unwrap_or_else(|e| format!("error: {:?}", e)))).await?;
                },

                Ok(message) = async {
                    if let Some(br) = &mut player_broadcast {
                        br.recv().await
                    } else {
                        pending().await
                    }
                }  => {
                    stream.send(Message::Text(serde_json::to_string(&message).unwrap_or_else(|e| format!("error: {:?}", e)))).await?;
                }
            }
        }

        Ok(())
    })))
}