use core::str;
use std::{collections::{HashMap, HashSet}, error::Error, fmt::Debug, sync::Arc};

use base64::{prelude::BASE64_STANDARD, Engine};
use dashmap::DashMap;
use games::{Game, PosData};
use google_auth_verifier::auth::{AuthVerifierClient, AuthenticationError};

use radip::{utils::{MapMeta, PowerMeta, ProvinceMeta}, Map, MapState, ProvinceAbbr, Unit};
use rocket::{form::Form, fs::{FileServer, TempFile}, http::{ContentType, Cookie, CookieJar, Status}, response::{content::RawHtml, Redirect, Responder}, serde::json::Json, tokio::io::AsyncReadExt, State};
use nanoid::nanoid;

mod games;

fn gen_id() -> String{
    nanoid!(16, &"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789".chars().collect::<Vec<_>>())
}

#[derive(Clone)]
struct AppState {
    pub users: Arc<DashMap<String, UserMeta>>,
    pub variants: Arc<DashMap<String, Variant>>,
    pub games: Arc<DashMap<String, Game>>
}

struct Variant {
    pub adj: Map,
    pub svg: String,
    pub meta: MapMeta,
    pub pos: PosData,
}

struct UserMeta {
    pub name: String,
}

#[macro_use] extern crate rocket;

#[litem::template("pages/components/head.html")]
struct HeadComponent;

#[litem::template("pages/components/header.html")]
struct HeaderComponent {
    pub user_name: String
}

#[get("/auth/<cred>")]
async fn auth(cred: &str, cookies: &CookieJar<'_>, app_state: &State<AppState>) -> Redirect {
    let mut verifier = AuthVerifierClient::new(None);
    let resp = verifier.verify_oauth_token(cred).await.map_err(|e| format!("{:?}", e))
        .and_then(|info| info.claims.get("sub")
            .and_then(|v| v.as_str().map(ToOwned::to_owned))
            .map(|sub| (info, sub))
            .ok_or("'sub' value not found".to_string()))
        .and_then(|(info, sub)| info.claims.get("name")
            .and_then(|v| v.as_str().map(ToOwned::to_owned))
            .map(|name| (name, sub))
            .ok_or("'name' value not found".to_string()));
    match resp {
        Ok((name, sub)) => {
            if !app_state.users.contains_key(&sub) {
                app_state.users.insert(sub.to_string(), UserMeta { name : name.to_string() });
            }

            cookies.add(("token", sub));
            
            Redirect::to(format!("/"))
        },
        Err(e) => {
            Redirect::to(format!("/error#{:?}", e))
        }
    }
}

 
#[litem::template("pages/signin.html")]
pub struct SigninPage {
    user_name: String,
}

#[get("/signin")]
fn signin() -> RawHtml<String> {
    RawHtml(SigninPage { user_name : "".to_string()}.render_string().unwrap())
}

#[litem::template("pages/home.html")]
pub struct HomePage {
    user_name: String,
}

#[get("/")]
fn home(cookies: &CookieJar<'_>, state: &State<AppState>) -> RawHtml<String> {
    let token = cookies.get("token").map(|c| c.value()).unwrap_or("");
    let name = state.users.get(token).map(|u| u.name.to_string()).unwrap_or("".to_string());

    RawHtml(HomePage {
        user_name: name
    }.render_string().unwrap())
}


fn encode_error<T: Debug>(e: T) -> String {
    let d = format!("{:?}", e);
    BASE64_STANDARD.encode(d.as_bytes()).replace("=", "%3D")
} 

#[litem::template("pages/error.html")]
struct ErrorPage<'a> {
    user_name: String,
    msg: &'a str,
    details: &'a str
}

#[get("/error?<msg>&<details>")]
fn error_page(cookies: &CookieJar<'_>, state: &State<AppState>, msg: Option<&str>, details: Option<&str>) -> RawHtml<String> {
    let token = cookies.get("token").map(|c| c.value()).unwrap_or("");
    let name = state.users.get(token).map(|u| u.name.to_string()).unwrap_or("".to_string());

    RawHtml(ErrorPage {
        user_name: name,
        msg: msg.unwrap_or("Internal Error"),
        details: str::from_utf8(&BASE64_STANDARD.decode(details.unwrap_or("")).unwrap_or(vec![]))
            .unwrap_or("")
    }.render_string().unwrap())
}


 
#[litem::template("pages/create_variant.html")]
struct CreateVariantPage {  
    user_name: String,
}


#[get("/variants/create")]
pub fn create_variant_page(cookies: &CookieJar<'_>, state: &State<AppState>) -> RawHtml<String> {
    let token = cookies.get("token").map(|c| c.value()).unwrap_or("");
    let user_name = state.users.get(token).map(|x| x.name.clone()).unwrap_or_else(String::new);

    RawHtml(CreateVariantPage {
        user_name
    }.render_string().unwrap())
}

#[get("/variants/<id>/map.svg")]
pub fn variant_svg(state: &State<AppState>, id: &str) -> Result<(ContentType, String), Status> {
    let variant = state.variants.get(id).ok_or(Status::NotFound)?;
    Ok((ContentType::SVG, variant.svg.clone()))
}

#[get("/variants/<id>/adj.json")]
pub fn variant_adj(state: &State<AppState>, id: &str) -> Result<Json<Map>, Status> {
    let variant = state.variants.get(id).ok_or(Status::NotFound)?;
    Ok(Json(variant.adj.clone()))
}

#[get("/variants/<id>/pos.json")]
pub fn variant_pos(state: &State<AppState>, id: &str) -> Result<Json<PosData>, Status> {
    let variant = state.variants.get(id).ok_or(Status::NotFound)?;
    Ok(Json(variant.pos.clone()))
}

#[get("/variants/<id>/meta.json")]
pub fn variant_meta(state: &State<AppState>, id: &str) -> Result<Json<MapMeta>, Status> {
    let variant = state.variants.get(id).ok_or(Status::NotFound)?;
    Ok(Json(variant.meta.clone()))
}


#[shuttle_runtime::main]
async fn rocket() -> shuttle_rocket::ShuttleRocket {
    let rocket = rocket::build().mount("/", routes![
        signin,
        auth,
        home,
        error_page,

        games::create_game_page,
        games::create_game_submit,
        games::game,
        games::game_meta,

        variant_adj, variant_svg, variant_pos, variant_meta,
        create_variant_page
    ])
    .mount("/static", FileServer::from(env!("CARGO_MANIFEST_DIR").to_owned() + "/static"))
        .manage(AppState {
            users: Arc::new(DashMap::new()),
            variants: Arc::new(DashMap::new()),
            games: Arc::new(DashMap::new())
        });

    Ok(rocket.into())
} 