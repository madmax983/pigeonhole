//! Your pigeon: its notes, the file it lives in, and the neighbours it shares
//! that file with but cannot see.

use autumn_web::AppState;
use autumn_web::prelude::*;
use autumn_web::reexports::{diesel, diesel_async};
use autumn_web::tenancy::Tenant;
use diesel_async::RunQueryDsl as _;
use holes::cell;
use serde::Deserialize;

use super::layout::{commas, page};
use crate::models::{Count, NewNote, Note, NoteRepository as _, PgNoteRepository};
use crate::{Pigeonhole, slot_of, tenant_name};

/// The most notes shown.
pub const SHOWN: usize = 200;

/// The pigeon page.
#[get("/pigeon")]
#[public]
pub async fn index(
    State(state): State<AppState>,
    tenant: Tenant,
    mut file: ShardedDb,
    mut db: Db,
    repo: PgNoteRepository,
    csrf: Option<CsrfToken>,
) -> AutumnResult<Markup> {
    #[derive(diesel::QueryableByName)]
    struct Name {
        #[diesel(sql_type = diesel::sql_types::Text)]
        name: String,
    }
    let p = Pigeonhole::of(&state)?;
    let name = tenant_name(&tenant.0)?;
    let s = slot_of(name.as_str());
    let (row, col) = cell(s);
    // Every row in the file, whoever it belongs to: raw SQL, no tenant filter.
    let all: Count = diesel::sql_query("SELECT COUNT(*) AS n FROM notes")
        .get_result(&mut *file)
        .await?;
    // What this pigeon can see: the tenant-scoped repository.
    let mine = repo.find_all().await?.len() as u64;
    let all = u64::try_from(all.n).unwrap_or(0);
    let theirs = all.saturating_sub(mine);
    let neighbours: Vec<Name> =
        diesel::sql_query("SELECT name FROM pigeons WHERE slot = ? AND name <> ? ORDER BY name")
            .bind::<diesel::sql_types::Integer, _>(i32::from(s.get()))
            .bind::<diesel::sql_types::Text, _>(name.as_str())
            .load(&mut *db)
            .await?;
    let key = p
        .fleet
        .key_for(name.as_str())
        .map_err(|e| AutumnError::bad_request_msg(e.to_string()))?;
    let mine_pct = (mine * 100).checked_div(all).unwrap_or(100);

    Ok(page(
        name.as_str(),
        csrf.as_ref().map(CsrfToken::token),
        &html! {
            h2 { code { (name) } }
            p class="lede" {
                "Your name hashes to slot " a href=(format!("/slot/{}", s.get())) { code { (format!("{:05}", s.get())) } }
                " (row " (row) ", column " (col) "). Everything you write goes into that slot's file"
                @if neighbours.is_empty() { ", which nobody else has landed in yet." }
                @else { ", which you share with " (neighbours.len()) " other pigeon"
                    @if neighbours.len() != 1 { "s" } "." }
            }
            div class="grid" {
                div class="panel" {
                    h3 { "Your notes" }
                    form hx-post="/pigeon/notes" hx-target="#notes" hx-swap="innerHTML" data-reset {
                        div style="display:flex;gap:8px" {
                            input type="text" id="note-body" name="body" placeholder="leave a note in your hole"
                                maxlength="500" required autocomplete="off";
                            button class="btn" type="submit" { "Write" }
                        }
                    }
                    div id="notes" hx-get="/pigeon/notes" hx-trigger="load" style="margin-top:12px" {
                        p class="hint" { "opening the file…" }
                    }
                }
                div {
                    div class="panel" {
                        h3 { "The file" }
                        dl class="kv" {
                            dt { "fleet name" } dd { code { (key.name()) } }
                            dt { "path" } dd { code { (p.fleet.path_of(&key).display()) } }
                            dt { "rows in it" } dd { (commas(all)) }
                            dt { "yours" } dd { (commas(mine)) }
                        }
                        div class="split" {
                            i class="mine" style=(format!("width:{mine_pct}%")) {}
                            i class="theirs" style=(format!("width:{}%", 100 - mine_pct)) {}
                        }
                        p class="hint" {
                            @if theirs == 0 {
                                "Every row in this file is yours."
                            } @else {
                                (commas(theirs)) " row" @if theirs != 1 { "s" } " in this file belong to "
                                "your neighbours. The repository is tenant-scoped, so they are in "
                                "the same file and you still cannot see them."
                            }
                        }
                    }
                    div class="panel" {
                        h3 { "Neighbours" }
                        @if neighbours.is_empty() {
                            p class="hint" { "Nobody else hashes to this slot. Yet." }
                        } @else {
                            div class="chips" {
                                @for n in &neighbours { a class="chip" href=(format!("/enter/{}", n.name)) { (n.name) } }
                            }
                            p class="hint" style="margin-top:8px" { "Click one to become it and look back at yourself." }
                        }
                    }
                }
            }
        },
    ))
}

/// Your notes, through the tenant-scoped, sharded repository.
#[get("/pigeon/notes")]
#[public]
pub async fn notes(repo: PgNoteRepository) -> AutumnResult<Markup> {
    let mut all = repo.find_all().await?;
    Ok(notes_list(&mut all))
}

fn notes_list(all: &mut [Note]) -> Markup {
    all.sort_by_key(|n| std::cmp::Reverse(n.id));
    html! {
        @if all.is_empty() {
            p class="empty" { "Nothing yet." }
        } @else {
            ul class="notes" {
                @for n in all.iter().take(SHOWN) {
                    li {
                        span class="when" { (n.created_at.format("%m-%d %H:%M:%S")) }
                        span { (n.body) }
                    }
                }
            }
        }
    }
}

/// A new note.
#[derive(Debug, Deserialize)]
pub struct NoteForm {
    /// What to write.
    pub body: String,
}

/// Write a note.
#[post("/pigeon/notes")]
#[public]
pub async fn add_note(
    repo: PgNoteRepository,
    tenant: Tenant,
    Form(form): Form<NoteForm>,
) -> AutumnResult<Markup> {
    let body = form.body.trim();
    if body.is_empty() || body.len() > 500 {
        return Err(AutumnError::unprocessable_msg(
            "a note is 1 to 500 characters",
        ));
    }
    repo.save(&NewNote {
        body: body.to_owned(),
        tenant_id: Some(tenant.0.clone()),
        created_at: chrono::Utc::now().naive_utc(),
    })
    .await?;
    let mut all = repo.find_all().await?;
    Ok(notes_list(&mut all))
}
