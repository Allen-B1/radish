use std::{collections::{HashMap, HashSet}, io::{Cursor, Read}};

use radip::{utils::MapMeta, Map, MapState, Orders, ProvinceAbbr};
use rocket::{form::Form, fs::{NamedFile, TempFile}, futures::{SinkExt, StreamExt}, http::{CookieJar, Status}, response::{content::RawHtml, Redirect}, serde::{json::Json, Deserialize, Serialize}, time::Duration, tokio::{io::AsyncReadExt, select, sync::broadcast}, State};
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

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all="snake_case")]
pub enum GamePhase {
    Spring,
    SpringRetreat,
    Fall,
    FallRetreat,
    Winter
}

pub struct GameState {
    pub map_state: MapState,
    pub year: u8,
    pub phase: GamePhase,
    pub next_adjudication: u64,
    
    // user id => power
    pub players: HashMap<String, String>,
    pub orders: Orders
}

#[derive(Deserialize, Clone)]
#[serde(rename_all="snake_case", tag = "type")]
enum InMessage {
    Auth { token: String }
}

#[derive(Serialize, Clone)]
#[serde(rename_all="snake_case", tag = "type")]
enum OutMessage {
    UpdatePlayers { players: HashSet<String> },

    Phase {
        year: u8,
        phase: GamePhase,
    },

    MovementAdj {
        year: u8,
        phase: GamePhase,
        orders: Orders,
        order_status: HashMap<String, bool>
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
                TimeUnit::Min => Duration::minutes(form.time_build as i64),
                TimeUnit::Hr => Duration::hours(form.time_build as i64)
            },
            time_mvmt: match form.time_mvmt_unit {
                TimeUnit::Min => Duration::minutes(form.time_mvmt  as i64),
                TimeUnit::Hr => Duration::hours(form.time_mvmt  as i64)
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

async fn handle_in_message(state: &AppState, game_id: &str, msg: InMessage, stream: &mut DuplexStream) -> Result<(), ()> {
    match msg {
        InMessage::Auth { token } => {
            if state.users.contains_key(&token) {
                let mut game = state.games.get_mut(game_id).ok_or(())?;   
                game.players.insert(token);
            }
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
                            
                            handle_in_message(&state, &id, msg, &mut stream).await;
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