use std::collections::{HashMap, HashSet};

use radip::{utils::{MapMeta, PowerMeta, ProvinceMeta}, MapState, Unit};
use rocket::{form::Form, fs::TempFile, http::{CookieJar, Status}, response::{content::{RawHtml, RawJson}, Redirect}, serde::json::Json, tokio::io::AsyncReadExt, State};

use crate::{encode_error, AppState, TempVariant};
use crate::{HeadComponent, HeaderComponent};

#[litem::template("pages/new_variant.html")]
struct NewVariantPage {
    user_name: String,
}

#[get("/variant/new")]
pub fn new_variant(cookies: &CookieJar<'_>, state: &State<AppState>) -> Result<RawHtml<String>, Redirect>  {
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

#[derive(FromForm)]
struct NewVariantForm<'a> {
    svg: TempFile<'a>,
    adj: &'a [u8],
    meta: &'a [u8]
}

fn extract_style<'a,'b>( prop: &'b str, style: &'a str) -> Option<&'a str> {
    let i = style.find(&format!("{}:", prop))?;
    let j = style[i+prop.len()+1..].find(";")?+prop.len()+1;
    let color = &style[i+prop.len()+1..j];
    Some(color)
}

#[post("/variant/new/files", data = "<form>")]
pub async fn new_variant_files(cookies: &CookieJar<'_>, state: &State<AppState>, form: Form<NewVariantForm<'_>>) -> Result<Redirect, Redirect>  {
    let token = cookies.get("token").map(|c| c.value()).unwrap_or("");
    let user = state.users.get(token).ok_or(Redirect::to("/signin"))?;

    let name = form.svg.name().unwrap_or("variant.svg");
    let mut name = name.split(".").next().unwrap_or("variant").to_string();
    
    let mut svg_content = String::new();
    form.svg.open().await
        .map_err(|e| Redirect::to(format!("/error?details={}", encode_error(e))))?
        .read_to_string(&mut svg_content).await
        .map_err(|e| Redirect::to(format!("/error?details={}", encode_error(e))))?;

    let document = roxmltree::Document::parse(&svg_content)
        .map_err(|e| Redirect::to(format!("/error?details={}&msg=Invalid+map+file", encode_error(e))))?;

    if let Some(r) = name.get_mut(0..1) {
        r.make_ascii_uppercase();
    }
        
    let mut temp_variant = TempVariant {
        name: name.to_string(),
        powers: HashMap::new(),
        starting_state: MapState {
            units: HashMap::new(),
            ownership: HashMap::new()
        },

        provinces: HashMap::new(),

        adj: serde_json::from_slice(form.adj)
            .map_err(|e| Redirect::to(format!("/error?details={}&msg=Invalid+adjacency+file", encode_error(e))))?,

        svg: svg_content.clone(),
    };

    for node in document.descendants()
        .filter(|n| n.attribute("id").is_some() && n.attribute("id").unwrap().starts_with("power-")) {
        let power_abbr = &node.attribute("id").unwrap()[6..];
        let style = node.attribute("style").unwrap_or("");
        let fill = match extract_style("fill", style) {
            Some(fill) => fill,
            None => continue
        };
        let stroke = match extract_style("stroke", style) {
            Some(stroke) => stroke,
            None => continue
        };

        temp_variant.powers.insert(power_abbr.to_string(), PowerMeta {
            name: node.text().unwrap_or(power_abbr).to_string(),
            tile_color: stroke.to_string(),
            sc_color: fill.to_string(),
        });
    }

    for (abbr, data) in temp_variant.adj.provinces.iter() {
        temp_variant.provinces.insert(abbr.to_string(), ProvinceMeta {
            name: abbr.to_string(),
            is_sc: false
        });

        // compute whether province is an SC
        if document.descendants()
            .filter(|n| n.attribute("id") == Some(&("sc-".to_string() + abbr)))
            .next().is_some() {
            
            temp_variant.provinces.get_mut(abbr).unwrap().is_sc = true;
        }

        // add ownership to starting state
        if let Some(n) = document.descendants()
            .filter(|n| n.attribute("id") == Some( abbr))
            .next() {

            if let Some(fill) = extract_style("fill", n.attribute("style").unwrap_or("")) {
                let power = temp_variant.powers.iter().filter(|(k, v)| v.tile_color == fill).next().map(|t| t.0);
                if let Some(power) = power {
                    temp_variant.starting_state.ownership.insert(abbr.to_string(), power.to_string());
                }
            }
        }

        if let Some(n) = document.descendants()
            .filter(|n| n.attribute("id") == Some(&format!("label-{}", abbr)))
            .next() {
            
            temp_variant.provinces.get_mut(abbr).unwrap().name = n.text().unwrap_or(abbr).to_string();
        }

        // add units to starting state
        let mut ids = data.coasts.iter().map(|c| format!("fleet-{}-{}", abbr, c)).collect::<HashSet<_>>();
        ids.insert(format!("fleet-{}", abbr));
        ids.insert(format!("army-{}", abbr));
        for node in document.descendants().filter(|n| n.attribute("id").is_some() && ids.contains(n.attribute("id").unwrap())) {
            let id = node.attribute("id").unwrap().split("-").collect::<Vec<_>>();
            let style = node.attribute("style").unwrap_or("");
            
            if let Some(fill) = extract_style("fill", style) {
                let power = temp_variant.powers.iter().filter(|(k, v)| v.tile_color == fill).next().map(|t| t.0);
                if let Some(power) = power {
                    let unit = match id[0] {
                        "fleet" => Unit::Fleet(power.to_string(), id.get(2).unwrap_or(&"").to_string()),
                        "army" => Unit::Army(power.to_string()),
                        _ => unreachable!()
                    };

                    temp_variant.starting_state.units.insert(abbr.to_string(), unit);
                }
            }
        }
    }

    let sub = token;
    state.temp_variants.insert(sub.to_string(), temp_variant);

    Ok(Redirect::to("/variant/new/2"))
}

#[litem::template("pages/new_variant_2.html")]
struct NewVariant2Page<'a> {
    user_name: &'a str,
    tempvar: &'a TempVariant,
}

#[get("/variant/new/2")]
pub fn new_variant_2(cookies: &CookieJar<'_>, state: &State<AppState>) -> Result<RawHtml<String>, Redirect>  {
    let token = cookies.get("token").map(|c| c.value()).unwrap_or("");
    let user = state.users.get(token).ok_or(Redirect::to("/signin"))?;

    let tempvar = state.temp_variants.get(token).ok_or(Redirect::to("/variant/new"))?;

    Ok(RawHtml(NewVariant2Page {
        user_name: &user.name,
        tempvar: &tempvar,
    }.render_string().unwrap()))
}

#[derive(FromForm)]
struct SubmitVariantForm {
    name: String,
    author: String,
    description: String
}
#[post("/variant/new/submit", data = "<form>")]
pub fn submit_variant(cookies: &CookieJar<'_>, state: &State<AppState>, form: Form<SubmitVariantForm>) -> Result<Redirect, Redirect> {
    let token = cookies.get("token").map(|c| c.value()).unwrap_or("");
    let user = state.users.get(token).ok_or(Redirect::to("/signin"))?;

    let tempvar = state.temp_variants.get(token).ok_or(Redirect::to("/error?msg=No+variant+stored"))?;
    let form: SubmitVariantForm = form.into_inner();

    let meta = MapMeta {
        name: form.name,
        author: form.author,

        powers: tempvar.powers.clone(),
        starting_state: tempvar.starting_state.clone(),
        provinces: tempvar.provinces.clone(),

        data: HashMap::from([
            ("uploader".to_string(), serde_json::Value::String(token.to_string())),
            ("description".to_string(), serde_json::Value::String(form.description))
        ]),
    };

    let id = meta.name.to_ascii_lowercase().chars()
        .filter(|c| c.is_ascii_alphanumeric() || c.is_ascii_whitespace())
        .map(|c| if c.is_ascii_whitespace() { '-' } else { c })
        .collect::<String>();

    if state.variants.contains_key(&id) {
        Err(Redirect::to(format!("/error?msg=Variant+{}+already+exists", &id)))?;
    }

    let tempvar = state.temp_variants.remove(token).ok_or(Redirect::to("/error?msg=Internal+error"))?.1;

    state.variants.insert(id.clone(), crate::Variant { adj: tempvar.adj, svg: tempvar.svg, meta: meta });

    Ok(Redirect::to(format!("/variant/{}", id)))
}

#[get("/variant/<id>/meta.json")]
pub fn variant_meta(cookies: &CookieJar<'_>, state: &State<AppState>, id: &str) -> Result<Json<MapMeta>, Status> {
    let token = cookies.get("token").map(|c| c.value()).unwrap_or("");
    let user_name = state.users.get(token).map(|x| x.name.clone()).ok_or("".to_string());

    if !state.variants.contains_key(id) {
        Err(Status::NotFound)?;
    }

    Ok(Json(state.variants.get(id).unwrap().meta.clone()))
}