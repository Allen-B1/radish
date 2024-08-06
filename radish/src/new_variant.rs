/*use std::collections::{HashMap, HashSet};

use radip::{utils::{MapMeta, PowerMeta, ProvinceMeta}, MapState, Unit};
use rocket::{form::Form, fs::TempFile, http::{ContentType, CookieJar, Status}, response::{content::{RawHtml, RawJson}, Redirect}, serde::json::Json, tokio::io::AsyncReadExt, State};

use crate::{encode_error, AppState, TempVariant, Variant};
use crate::{HeadComponent, HeaderComponent};
use base64::{Engine, prelude::BASE64_STANDARD};

#[litem::template("pages/new_variant.html")]
struct NewVariantPage {
    user_name: String,
}

#[get("/variant/new")]
pub fn new_variant_page(cookies: &CookieJar<'_>, state: &State<AppState>) -> Result<RawHtml<String>, Redirect>  {
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
    adj: TempFile<'a>
}

fn extract_style<'a,'b>( prop: &'b str, style: &'a str) -> Option<&'a str> {
    let i = style.find(&format!("{}:", prop))?;
    let j = style[i+prop.len()+1..].find(";")?+i+prop.len()+1;
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

    let mut adj_str = String::new();
    form.adj.open().await
            .map_err(|e| Redirect::to(format!("/error?details={}", encode_error(e))))?
            .read_to_string(&mut adj_str).await
            .map_err(|e| Redirect::to(format!("/error?details={}", encode_error(e))))?;
        
    let mut temp_variant = TempVariant {
        name: name.to_string(),
        powers: HashMap::new(),
        starting_state: MapState {
            units: HashMap::new(),
            ownership: HashMap::new()
        },
        home_sc: HashMap::new(),

        provinces: HashMap::new(),

        adj: serde_json::from_str(&adj_str)
            .map_err(|e| Redirect::to(format!("/error?details={}&msg=Invalid+adjacency+file", encode_error(e))))?,

        svg: svg_content.clone(),
    };

    // find powers
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
            sc_color: stroke.to_string(),
            tile_color: fill.to_string(),
        });
    }

    for (abbr, data) in temp_variant.adj.provinces.iter() {
        temp_variant.provinces.insert(abbr.to_string(), ProvinceMeta {
            name: abbr.to_string(),
            is_sc: false
        });

        // compute whether province is an SC
        if let Some(sc) = document.descendants()
            .filter(|n| n.attribute("id") == Some(&("sc-".to_string() + abbr)))
            .next() {

            temp_variant.provinces.get_mut(abbr).unwrap().is_sc = true;

            // compute ownership
            if let Some(fill) = extract_style("fill", sc.attribute("style").unwrap_or("")) {
                if let Some((power, _)) = temp_variant.powers.iter().filter(|(k, v)| v.sc_color == fill).next() {
                    temp_variant.starting_state.ownership.insert(abbr.to_string(), power.to_string());
                    temp_variant.home_sc.insert(abbr.to_string(), power.to_string());
                }
            }
        }

        if let Some(sc) = document.descendants()
            .filter(|n| n.attribute("id") == Some(&format!("sc-{}-home", abbr)))
            .next() {
            
            if let Some(fill) = extract_style("fill", sc.attribute("style").unwrap_or("")) {
                match temp_variant.powers.iter().filter(|(k, v)| v.sc_color == fill).next() {
                    Some((power, _)) =>  { temp_variant.home_sc.insert(abbr.to_string(), power.to_string()); } ,
                    None => { temp_variant.home_sc.remove(abbr); }
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
            println!("{} {}", abbr, node.attribute("id").unwrap());  
            let id = node.attribute("id").unwrap().split("-").collect::<Vec<_>>();
            let style = node.attribute("style").unwrap_or("");
            
            if let Some(fill) = extract_style("fill", style) {
                let power = temp_variant.powers.iter().filter(|(k, v)| v.sc_color == fill).next().map(|t| t.0);
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
    let id = form.name.to_ascii_lowercase().chars()
        .filter(|c| c.is_ascii_alphanumeric() || c.is_ascii_whitespace())
        .map(|c| if c.is_ascii_whitespace() { '-' } else { c })
        .collect::<String>();

    drop(tempvar);
    if state.variants.contains_key(&id) {
        Err(Redirect::to(format!("/error?msg=Variant+{}+already+exists", &id)))?;
    }

    let tempvar = state.temp_variants.remove(token).ok_or(Redirect::to("/error?msg=Internal+error"))?.1;

    let meta = MapMeta {
        name: form.name,
        author: form.author,

        powers: tempvar.powers,
        home_sc: tempvar.home_sc,
        starting_state: tempvar.starting_state,

        provinces: tempvar.provinces,

        data: HashMap::from([
            ("uploader".to_string(), serde_json::Value::String(user.name.to_string())),
            ("description".to_string(), serde_json::Value::String(form.description))
        ]),
    };

    state.variants.insert(id.clone(), crate::Variant { adj: tempvar.adj, svg: tempvar.svg, meta: meta });

    Ok(Redirect::to(format!("/variant/{}", id)))
}

#[get("/variant/<id>/meta.json")]
pub fn variant_meta(state: & State<AppState>, id: &str) -> Result<Json<MapMeta>, Status> {
    let variant = state.variants.get(id).ok_or(Status::NotFound)?;

    Ok(Json(variant.meta.clone()))
}

#[get("/variant/<id>/map.svg")]
pub fn variant_map(state: & State<AppState>, id: &str) -> Result<(ContentType, String), Status> {
    let variant = state.variants.get(id).ok_or(Status::NotFound)?;

    Ok((ContentType::SVG, variant.svg.clone()))
}

#[litem::template("pages/variant.html")]
struct VariantPage<'a> {
    variant: &'a Variant,
    variant_id: &'a str,
    user_name: String
}

#[get("/variant/<id>")]
pub fn variant_page(cookies: &CookieJar<'_>, state: &State<AppState>, id: &str) -> Result<RawHtml<String>, Status> {
    let token = cookies.get("token").map(|c| c.value()).unwrap_or("");
    let user_name = state.users.get(token).map(|x| x.name.clone()).unwrap_or_else(String::new);
    let variant = state.variants.get(id).ok_or(Status::NotFound)?;

    Ok(RawHtml(VariantPage {
        variant: &variant,
        variant_id: id,
        user_name: user_name
    }.render_string().unwrap()))
}
*/