//! The board: all 16,384 slots, who lives in each, which files are open.

use std::convert::Infallible;
use std::sync::Arc;

use autumn_web::AppState;
use autumn_web::prelude::*;
use autumn_web::reexports::axum::response::Response;
use autumn_web::reexports::{diesel, diesel_async};
use autumn_web::session::Session;
use autumn_web::sse::{Sse, from_subscriber};
use diesel_async::RunQueryDsl as _;
use holes::{Bird, Slot, TenantName, cell, expected_files};
use serde::Deserialize;
use tokio_stream::Stream;

use super::TENANT_KEY;
use super::layout::{commas, page, stat};
use crate::board::{Picture, TOPIC};
use crate::flock::{self, FlockReport};
use crate::{Pigeonhole, slot_of};

/// How many pigeons the flock button releases.
pub const FLOCK: u32 = 3_000;

/// The board.
#[get("/")]
#[public]
pub async fn index(State(state): State<AppState>, csrf: Option<CsrfToken>) -> AutumnResult<Markup> {
    let p = Pigeonhole::of(&state)?;
    Ok(page(
        "The board",
        csrf.as_ref().map(CsrfToken::token),
        &html! {
            h2 { "The board" }
            p class="lede" {
                "Every square is one of the fleet's 16,384 slots, and every slot is one SQLite "
                "file. A pigeon's name hashes to a slot; any number of pigeons may share one. "
                "Release a flock and watch the holes fill — and start to double up long before "
                "they run out."
            }
            div class="grid" {
                div class="panel" {
                    h3 { "16,384 slots · 128 × 128" }
                    div class="board-wrap" {
                        canvas id="board" width="640" height="640"
                            aria-label="The fleet's 16,384 slots, coloured by how many tenants each holds" {}
                    }
                    div class="hover" id="hover" { "point at a slot" }
                    div class="legend" {
                        span { i style="background:var(--empty)" {} "empty (no file)" }
                        span { i style="background:var(--neck)" {} "one pigeon" }
                        span { i style="background:var(--neck-2)" {} "two" }
                        span { i style="background:var(--hot)" {} "three or more" }
                        span { i style="background:#fff" {} "file open now" }
                    }
                }
                div hx-ext="sse" sse-connect="/stream" {
                    div id="panel" hx-get="/_panel" hx-trigger="sse:message" hx-swap="innerHTML" {
                        (panel_body(&p.board.picture(), &p.flock.report()))
                    }
                    (found_form(None))
                }
            }
        },
    ))
}

/// The board's data, for the canvas.
#[get("/_board.json")]
#[public]
pub async fn picture(State(state): State<AppState>) -> AutumnResult<Json<Picture>> {
    Ok(Json(Pigeonhole::of(&state)?.board.picture()))
}

/// The side panels, for the live feed.
#[get("/_panel")]
#[public]
pub async fn panel(State(state): State<AppState>) -> AutumnResult<Markup> {
    let p = Pigeonhole::of(&state)?;
    Ok(panel_body(&p.board.picture(), &p.flock.report()))
}

/// One message per redraw-worthy change.
#[get("/stream")]
#[public]
pub async fn stream(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<autumn_web::sse::Event, Infallible>>> {
    from_subscriber(state.channels().subscribe(TOPIC))
}

/// The form behind "hatch a pigeon".
#[derive(Debug, Deserialize)]
pub struct FoundForm {
    /// The pigeon's name.
    pub name: String,
}

/// Hatch a pigeon and become it.
#[post("/pigeons")]
#[public]
pub async fn found(
    State(state): State<AppState>,
    session: Session,
    Form(form): Form<FoundForm>,
) -> AutumnResult<Response> {
    let p = Pigeonhole::of(&state)?;
    let name = match TenantName::parse(form.name.trim()) {
        Ok(n) => n,
        Err(e) => return Ok(html! { p class="err" { (e) } }.into_response()),
    };
    let control = state
        .pool()
        .ok_or_else(|| AutumnError::service_unavailable_msg("no control database"))?;
    let bird = Bird {
        notes: vec![format!("{name} hatched here")],
        name: name.clone(),
    };
    if let Err(e) = flock::land(&p.fleet, control, &p.board, &bird).await {
        return Ok(html! { p class="err" { (e) } }.into_response());
    }
    session.insert(TENANT_KEY, name.as_str()).await;
    Ok(html! { p class="ok" { "Hatched " (name) "…" } }.hx_redirect("/pigeon"))
}

/// Become a registered pigeon.
#[get("/enter/{name}")]
#[public]
pub async fn enter(
    session: Session,
    mut db: Db,
    Path(raw): Path<String>,
) -> AutumnResult<Response> {
    let name = crate::tenant_name(&raw)?;
    let known: crate::models::Count =
        diesel::sql_query("SELECT COUNT(*) AS n FROM pigeons WHERE name = ?")
            .bind::<diesel::sql_types::Text, _>(name.as_str())
            .get_result(&mut *db)
            .await?;
    if known.n == 0 {
        return Err(AutumnError::not_found_msg(format!(
            "no pigeon called {name}"
        )));
    }
    session.insert(TENANT_KEY, name.as_str()).await;
    Ok(Redirect::to("/pigeon").into_response())
}

/// Release a flock.
#[post("/flock")]
#[public]
pub async fn release(State(state): State<AppState>) -> AutumnResult<Markup> {
    let p = Pigeonhole::of(&state)?;
    let control = state
        .pool()
        .cloned()
        .ok_or_else(|| AutumnError::service_unavailable_msg("no control database"))?;
    p.flock.skip_names(p.board.picture().pigeons);
    flock::release(
        p.fleet.clone(),
        control,
        Arc::clone(&p.board),
        Arc::clone(&p.flock),
        FLOCK,
    );
    Ok(panel_body(&p.board.picture(), &p.flock.report()))
}

/// One slot: who lives there.
#[get("/slot/{n}")]
#[public]
pub async fn slot(
    State(state): State<AppState>,
    mut db: Db,
    Path(n): Path<u32>,
) -> AutumnResult<Markup> {
    #[derive(diesel::QueryableByName)]
    struct Name {
        #[diesel(sql_type = diesel::sql_types::Text)]
        name: String,
    }
    let p = Pigeonhole::of(&state)?;
    let s = Slot::new(n).map_err(|e| AutumnError::bad_request_msg(e.to_string()))?;
    let names: Vec<Name> =
        diesel::sql_query("SELECT name FROM pigeons WHERE slot = ? ORDER BY name")
            .bind::<diesel::sql_types::Integer, _>(i32::from(s.get()))
            .load(&mut *db)
            .await?;
    let key = p
        .fleet
        .key_for_name(&s.db_name())
        .map_err(|e| AutumnError::bad_request_msg(e.to_string()))?;
    let path = p.fleet.path_of(&key);
    let (row, col) = cell(s);
    Ok(page(
        &s.db_name(),
        None,
        &html! {
            h2 { "Slot " code { (format!("{:05}", s.get())) } }
            p class="lede" { "Row " (row) ", column " (col) " on the board." }
            div class="panel" {
                dl class="kv" {
                    dt { "fleet name" } dd { code { (s.db_name()) } }
                    dt { "file" } dd {
                        @if p.fleet.exists(&key) { code { (path.display()) } } @else { "none yet — no pigeon has landed here" }
                    }
                    dt { "pigeons" } dd { (names.len()) }
                }
                @if !names.is_empty() {
                    h3 style="margin-top:16px" { "Who lives here" }
                    div class="chips" {
                        @for n in &names { a class="chip" href=(format!("/enter/{}", n.name)) { (n.name) } }
                    }
                }
            }
            p { a href="/" { "Back to the board" } }
        },
    ))
}

fn found_form(prefill: Option<&str>) -> Markup {
    html! {
        div class="panel" {
            h3 { "Hatch a pigeon" }
            form class="row" hx-post="/pigeons" hx-target="#found-msg" hx-swap="innerHTML" {
                input type="text" id="pigeon-name" name="name" placeholder="dusk-roller" value=(prefill.unwrap_or(""))
                    autocomplete="off" spellcheck="false" maxlength="40" required;
                button class="btn" type="submit" { "Hatch" }
            }
            div id="found-msg" {}
            p class="hint" { "Its name decides its slot. a–z, 0–9 and single hyphens." }
        }
    }
}

/// The side panels.
pub fn panel_body(pic: &Picture, flock: &FlockReport) -> Markup {
    let expected = expected_files(pic.pigeons);
    #[allow(clippy::cast_precision_loss)]
    let ratio = if pic.pigeons == 0 {
        0.0
    } else {
        pic.occupied as f64 / pic.pigeons as f64
    };
    let pct = (flock.done * 100)
        .checked_div(flock.total)
        .unwrap_or(0)
        .min(100);
    html! {
        div class="panel" {
            h3 { "Occupancy" }
            div class="stats" {
                (stat(commas(pic.pigeons), "pigeons", true))
                (stat(commas(pic.occupied), "slot files", false))
                (stat(commas(pic.shared), "shared files", pic.shared > 0))
                (stat(pic.most, "most in one slot", false))
            }
            p class="hint" style="margin-top:10px" {
                "Uniform hashing predicts " b { (format!("{expected:.0}")) } " files for "
                (commas(pic.pigeons)) " pigeons; there are " b { (commas(pic.occupied)) } ", so "
                (format!("{:.2}", ratio)) " files per pigeon."
            }
        }
        div class="panel" {
            h3 { "Open files" }
            div class="stats" {
                (stat(pic.open_now, "open now", pic.open_now > 0))
                (stat(pic.peak_open, "peak", false))
                (stat(commas(pic.opens), "opens", false))
            }
        }
        div class="panel" id="flock" {
            h3 { "The flock" }
            p class="hint" {
                (commas(u64::from(FLOCK))) " pigeons with one to three notes each, eight slot files "
                "written at a time."
            }
            @if flock.releases > 0 {
                div class="bar" { i style=(format!("width:{pct}%")) {} }
                div class="stats" {
                    (stat(format!("{} / {}", commas(flock.done), commas(flock.total)), "landed", flock.running))
                    (stat(commas(flock.notes), "notes", false))
                    (stat(flock.peak_open, "peak open", false))
                    (stat(commas(flock.failed), "failed", flock.failed > 0))
                }
                @if let Some(e) = &flock.last_error { p class="err" { (e) } }
            }
            div style="margin-top:12px" {
                @if flock.running {
                    button class="btn" disabled { "In the air…" }
                } @else {
                    button class="btn" hx-post="/flock" hx-target="#panel" hx-swap="innerHTML" {
                        @if flock.releases == 0 { "Release a flock" } @else { "Release another" }
                    }
                }
            }
        }
    }
}

/// Where a pigeon would land, for the hatch form's hint (and tests).
#[must_use]
pub fn landing(name: &TenantName) -> (Slot, (u16, u16)) {
    let s = slot_of(name.as_str());
    (s, cell(s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_panel_reports_expected_against_actual() {
        let pic = Picture {
            occupancy: String::new(),
            open: vec![],
            pigeons: 3_000,
            occupied: 2_751,
            shared: 230,
            most: 3,
            open_now: 4,
            peak_open: 40,
            opens: 2_800,
        };
        let html = panel_body(&pic, &FlockReport::default()).into_string();
        assert!(html.contains("2,751"));
        assert!(html.contains("Release a flock"));
        assert!(html.contains("predicts"));
    }

    #[test]
    fn landing_is_the_routers_slot() {
        let n = TenantName::parse("dusk-roller").unwrap();
        let (s, (r, c)) = landing(&n);
        assert_eq!(s, slot_of("dusk-roller"));
        assert_eq!(u32::from(r) * 128 + u32::from(c), u32::from(s.get()));
    }
}
