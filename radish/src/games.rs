use std::{collections::{HashMap, HashSet}, hash::Hash, io::{Cursor, Read}, time::{SystemTime, UNIX_EPOCH}};
use rand::prelude::*;
use radip::{adjudicate, base::{Hold, Move}, utils::{apply_adjudication, MapMeta, RetreatOptions}, Map, MapState, Orders, ProvinceAbbr, Unit};
use rocket::{build, form::Form, fs::{NamedFile, TempFile}, futures::{SinkExt, StreamExt}, http::{CookieJar, Status}, response::{content::RawHtml, Redirect}, serde::{json::Json, Deserialize, Serialize}, tokio::{io::AsyncReadExt, select, sync::broadcast, time::{Duration, Instant}}, State};
use tokio::time;
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

#[derive(Clone, Serialize, Deserialize, Copy, PartialEq, Eq, Hash)]
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
enum InMessage {
    Auth { token: String },
    Orders { orders: Orders },
    Builds { builds: Builds }
}

#[derive(Serialize, Clone)]
#[serde(rename_all="snake_case", tag = "type")]
enum OutMessage {
    Error { msg : String },
    UpdatePlayers { players: HashSet<String> },

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
    pub players: HashSet<String>,
    pub state: Option<GameState>
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

    let meta: MapMeta = serde_json::from_reader(zip.by_name("meta.json")
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

        let adj: Map = serde_json::from_reader(zip.by_name("adj.json")
            .map_err(|e| Redirect::to(format!("/error?msg=Invalid+variant+file&details={}", encode_error(e))))?)
            .map_err(|e| Redirect::to(format!("/error?msg=Invalid+variant+file&details={}", encode_error(e))))?;

        let pos: PosData =  serde_json::from_reader(zip.by_name("pos.json")
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
        players: HashSet::from([token.to_string()]),
        state: None
    });

    Ok(Redirect::to(format!("/games/{}", game_id)))
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
    {
        let mut game = state.games.get_mut(&game_id).unwrap();
        let variant = state.variants.get(&game.meta.variant).unwrap();
        let adj_time = Instant::now() + game.meta.time_mvmt;
        let mut players = HashMap::new();
        let mut powers = variant.meta.powers.keys().map(|x| x.clone()).collect::<Vec<String>>();
        powers.shuffle(&mut thread_rng());

        for player in game.players.iter() {
            players.insert(player.clone(), powers.pop().unwrap());
        }

        game.state = Some(GameState {
            year: 1,
            phase: GamePhase::Fall,
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
        let game = state.games.get_mut(&game_id).unwrap();
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
            let mut new_mstate = gstate.current_state().clone();
            let retreats = apply_adjudication(&variant.adj, &mut new_mstate, orders, &order_status);

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
            for (prov, order) in gstate.orders.get(&(gstate.year, gstate.phase)).unwrap().iter() {
                let mov = match order.downcast_ref::<Move>() {
                    None => continue,
                    Some(mov) => mov
                };
                let retreats = match mvmt_info.retreats.get(prov) {
                    None => continue,
                    Some(r) => r
                };
                if retreats.dest.contains(&(mov.dest.0.clone(), mov.dest.1.clone())) {
                    let new_unit = match &retreats.src {
                        Unit::Army(natl) => Unit::Army(natl.clone()),
                        Unit::Fleet(natl, _) => Unit::Fleet(natl.clone(), mov.dest.1.clone())
                    };
                    new_mstate.units.insert(mov.dest.0.clone(), new_unit);
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

async fn handle_in_message(state: &AppState, game_id: &str, token: &mut String, msg: InMessage, stream: &mut DuplexStream) -> Result<(), ()> {
    let mut send = |msg: OutMessage| {
        stream.send(Message::Text(serde_json::to_string(&msg).unwrap()));
    };

    match msg {
        InMessage::Auth { token: tok } => {
            if state.users.contains_key(&tok) {
                let mut game = state.games.get_mut(game_id).ok_or(())?;   
                game.players.insert(tok.clone());
                game.broadcast.send(OutMessage::UpdatePlayers { players: game.players.clone() });

                let variant = state.variants.get(&game.meta.variant).unwrap();
                if game.players.len() == variant.meta.powers.len() {
                    // start game
                    let state_clone = state.clone();
                    let game_id_clone = game_id.to_string();
                    tokio::spawn(async move {
                        game_thread(state_clone, game_id_clone)
                    });
                }

                *token = tok;
            }
        },
        InMessage::Builds { builds } => {
            let mut game = state.games.get_mut(game_id).ok_or(())?;   
            if game.state.is_none() {
                send(OutMessage::Error { msg: "Game not started".to_string() });
                return Ok(())
            }
            if !state.users.contains_key(token) || !game.players.contains(token) {
                send(OutMessage::Error { msg: "Not authenticated".to_string() });
                return Ok(())
            }

            let gstate = game.state.as_mut().unwrap();
            let power = &gstate.players[&*token];
            for (prov, unit) in builds {
                if gstate.current_state().units.contains_key(&prov) {
                    send(OutMessage::Error { msg: format!("Build location {} is occupied", &prov) });
                    return Ok(())           
                }
                if gstate.current_state().ownership.get(&prov) != Some(&unit.nationality()) || unit.nationality() != *power {
                    send(OutMessage::Error { msg: format!("Build location {} is not owned by you", &prov) });
                    return Ok(())
                }

                let builds = gstate.builds.entry((gstate.year, gstate.phase)).or_insert(HashMap::new());
                builds.insert(prov, unit);
            }
        },
        InMessage::Orders { orders } => {
            let mut game = state.games.get_mut(game_id).ok_or(())?;   
            if game.state.is_none() {
                send(OutMessage::Error { msg: "Game not started".to_string() });
                return Ok(())
            }
            if !state.users.contains_key(token) || !game.players.contains(token) {
                send(OutMessage::Error { msg: "Not authenticated".to_string() });
                return Ok(())
            }

            let gstate = game.state.as_mut().unwrap();
            let power = &gstate.players[&*token];

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
                            
                            handle_in_message(&state, &id, &mut token, msg, &mut stream).await;
                        },
                        Message::Ping(data) => {
                            stream.send(Message::Pong(data)).await?;
                        },
                        _ => {}
                    }
                },

                Ok(message) = broadcast.recv() => {
                    stream.send(Message::Text(serde_json::to_string(&message).unwrap_or_else(|e| format!("error: {:?}", e)))).await?;
                }
            }
        }

        Ok(())
    })))
}