use core::str;
use std::{collections::{HashMap, HashSet}, error::Error, fmt::Debug};

use base64::{prelude::BASE64_STANDARD, Engine};
use dashmap::DashMap;
use google_auth_verifier::auth::{AuthVerifierClient, AuthenticationError};
mod new_variant;
use radip::{utils::{MapMeta, PowerMeta, ProvinceMeta}, Map, MapState, ProvinceAbbr, Unit};
use rocket::{form::Form, fs::{FileServer, TempFile}, http::{Cookie, CookieJar}, response::{content::RawHtml, Redirect, Responder}, tokio::io::AsyncReadExt, State};

struct AppState {
    pub users: DashMap<String, UserMeta>,
    pub temp_variants: DashMap<String, TempVariant>,
    pub variants: DashMap<String, Variant>,
}

struct TempVariant {
    pub name: String,

    pub powers: HashMap<String, PowerMeta>,
    pub starting_state: MapState,
    pub home_sc: HashMap<ProvinceAbbr, String>,
    pub provinces: HashMap<ProvinceAbbr, ProvinceMeta>,

    pub adj: Map,
    pub svg: String,
}

struct Variant {
    pub adj: Map,
    pub svg: String,
    pub meta: MapMeta,
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


#[get("/game/<id>")]
fn game(id: &str) -> RawHtml<&'static str> {
    RawHtml(include_str!("../pages/game.html"))
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
fn error_page(cookies: &CookieJar<'_>, state: &State<AppState>, msg: &str, details: Option<&str>) -> RawHtml<String> {
    let token = cookies.get("token").map(|c| c.value()).unwrap_or("");
    let name = state.users.get(token).map(|u| u.name.to_string()).unwrap_or("".to_string());

    RawHtml(ErrorPage {
        user_name: name,
        msg: msg,
        details: str::from_utf8(&BASE64_STANDARD.decode(details.unwrap_or("")).unwrap_or(vec![]))
            .unwrap_or("")
    }.render_string().unwrap())
}

#[shuttle_runtime::main]
async fn rocket() -> shuttle_rocket::ShuttleRocket {
    let rocket = rocket::build().mount("/", routes![
        game,
        signin,
        auth,
        home,
        error_page,

        new_variant::create_variant_page
    ])
    .mount("/static", FileServer::from(env!("CARGO_MANIFEST_DIR").to_owned() + "/static"))
        .manage(AppState {
            users: DashMap::new(),
            temp_variants: DashMap::new(),
            variants: DashMap::new()
        });

    Ok(rocket.into())
} 