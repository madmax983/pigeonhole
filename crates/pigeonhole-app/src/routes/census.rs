//! The census page.

use autumn_web::prelude::*;
use autumn_web::sharding::Shards;

use super::layout::{commas, page};
use crate::census::{self, Taken};

/// The census page: a button, because a census reads every file on disk.
#[get("/census")]
#[public]
pub async fn index(csrf: Option<CsrfToken>) -> AutumnResult<Markup> {
    Ok(page(
        "Census",
        csrf.as_ref().map(CsrfToken::token),
        &html! {
            h2 { "Census" }
            p class="lede" {
                "Read every slot file on disk, count who is in each, and check every row "
                "against the router: a pigeon's rows belong in exactly one file, the one its "
                "name hashes to. A row anywhere else is a stray. Keep the board open in another "
                "tab and watch the sweep."
            }
            div class="panel" id="census" {
                button class="btn" hx-post="/census" hx-target="#census" hx-swap="innerHTML"
                    hx-indicator="#census-busy" { "Take the census" }
                span id="census-busy" class="hint htmx-indicator" style="margin-left:10px" { "reading every file…" }
            }
        },
    ))
}

/// Take the census. It uses `LazyDb`, so it takes the control connection only
/// after the sweep.
#[post("/census")]
#[public]
pub async fn take(shards: Shards, control: LazyDb) -> AutumnResult<Markup> {
    let taken = census::take(&shards, control).await?;
    Ok(result(&taken))
}

/// The result.
pub fn result(t: &Taken) -> Markup {
    let c = &t.census;
    html! {
        div class=(if t.clean() { "verdict good" } else { "verdict bad" }) {
            @if t.clean() { "Every row is in its pigeon's slot." }
            @else { "The census disagrees with the router." }
        }
        p class="hint" {
            (commas(c.files)) " files read in " (t.took.as_millis()) " ms · "
            (commas(c.tenants)) " pigeons found · " (commas(c.notes)) " notes · "
            (commas(t.registered)) " pigeons registered"
        }
        table class="occ" style="margin-top:14px" {
            thead { tr { th { "pigeons in a file" } th { "files" } } }
            tbody {
                @for (k, n) in &c.by_occupancy {
                    tr { td { (k) } td { (commas(*n)) } }
                }
            }
        }
        @if !c.strays.is_empty() {
            h3 style="margin-top:16px" { "Strays" }
            ul {
                @for s in c.strays.iter().take(50) {
                    li { code { (s.tenant) } " in " code { (s.found_in.db_name()) } ", belongs in " code { (s.belongs_in.db_name()) } }
                }
            }
        }
        @if !t.missing.is_empty() {
            h3 style="margin-top:16px" { "Registered, but no rows in their file" }
            p class="mono" { @for m in &t.missing { (m) " " } }
        }
        @if !t.unknown.is_empty() {
            h3 style="margin-top:16px" { "In a file, but not registered" }
            p class="mono" { @for m in &t.unknown { (m) " " } }
        }
        @if !t.unreadable.is_empty() {
            h3 style="margin-top:16px" { "Unreadable" }
            ul { @for (n, e) in &t.unreadable { li { code { (n) } ": " (e) } } }
        }
        div style="margin-top:14px" {
            button class="btn quiet" hx-post="/census" hx-target="#census" hx-swap="innerHTML" { "Take it again" }
        }
    }
}
