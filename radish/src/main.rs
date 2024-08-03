use std::collections::HashMap;

use dashmap::DashMap;
use google_auth_verifier::auth::{AuthVerifierClient, AuthenticationError};
use rocket::{fs::FileServer, http::{Cookie, CookieJar}, response::{content::RawHtml, Redirect, Responder}, State};

struct AppState {
    pub users: DashMap<String, UserMeta>,
}

struct UserMeta {
    pub name: String,
}

struct MapInfo {
    map: radip::Map,
    meta: radip::utils::MapMeta,
    svg: String,
}

#[macro_use] extern crate rocket;

#[litem::template("pages/components/head.html")]
struct HeadComponent;

#[litem::template("pages/components/header.html")]
struct HeaderComponent {
    pub user_name: String
}

#[get("/auth/<cred>")]
async fn auth(cred: String, cookies: &CookieJar<'_>, app_state: &State<AppState>) -> Redirect {
    let mut verifier = AuthVerifierClient::new(None);
    let resp = verifier.verify_oauth_token(&cred).await.map_err(|e| format!("{:?}", e))
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
fn home(cookies: &CookieJar<'_>, state: &State<AppState>) -> Result<RawHtml<String>, Redirect> {
    let token = cookies.get("token").map(|c| c.value()).unwrap_or("");
    match state.users.get(token) {
        None => Err(Redirect::to("/signin")),  
        Some(user) => {
            Ok(RawHtml(HomePage {
                user_name: user.name.clone()
            }.render_string().unwrap()))
        } 
    }
}

#[litem::template("pages/new_variant.html")]
struct NewVariantPage {
    user_name: String,
}

#[get("/variant/new")]
fn new_variant(cookies: &CookieJar<'_>, state: &State<AppState>) -> Result<RawHtml<String>, Redirect>  {
    let token = cookies.get("token").map(|c| c.value()).unwrap_or("");
    match state.users.get(token) {
        None => Err(Redirect::to("/signin")),  
        Some(user) => {
            Ok(RawHtml(NewVariantPage {
                user_name: user.name.clone()
            }.render_string().unwrap()))
        } 
    }
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![
        game,
        signin,
        auth,
        home,
        new_variant
    ])
    .mount("/static", FileServer::from("./static"))
        .manage(AppState {
            users: DashMap::new()
        })
} 