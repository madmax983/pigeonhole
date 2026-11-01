//! How it works.

use autumn_web::prelude::*;

use super::layout::page;

/// The about page.
#[get("/about")]
#[public]
pub async fn show() -> AutumnResult<Markup> {
    Ok(page(
        "How it works",
        None,
        &html! {
            div class="prose" {
                h2 { "How it works" }
                p {
                    "autumn-web 0.9's database fleet has two modes. In tenant mode every tenant is "
                    "its own SQLite file. In slot mode — this one — there are exactly 16,384 files "
                    "at most, one per routing slot, and a tenant's id is hashed to choose its slot. "
                    "Any number of tenants can share a slot file. The file count is bounded however "
                    "many tenants arrive, and a slot is the unit that moves between hosts."
                }
                h3 { "Sharing a file" }
                p {
                    "Two pigeons in one slot write into one file. What keeps them apart is the "
                    "repository: it is declared " code { "#[repository(Note, tenant_scoped, sharded)]" }
                    ", so every query filters on the request's tenant. Your pigeon's page shows both "
                    "numbers: every row in your file, and the ones you can see."
                }
                h3 { "The board" }
                p {
                    "Each square is a slot, drawn at row slot ÷ 128, column slot mod 128 — a "
                    "bijection " code { "verus/src/grid.rs" } " proves. Colour is how many pigeons "
                    "the registry has in that slot; white is a file the fleet has open right now, "
                    "reported by a lifecycle hook on the fleet."
                }
                h3 { "How full it gets" }
                p {
                    "n names hashed uniformly into 16,384 slots hit about 16,384 · (1 − (1 − "
                    "1/16,384)ⁿ) distinct slots. Three thousand pigeons already share about two "
                    "hundred files; by eleven thousand, half of all slots are taken. The board "
                    "shows the prediction beside the real count."
                }
                h3 { "The census" }
                p {
                    "The census runs " code { "Shards::each_shard" } " over every slot file on disk, "
                    "counts each tenant's rows in each file, and checks every row against the "
                    "router. Its totals are proved to conserve the per-file reports in "
                    code { "verus/src/census.rs" } "."
                }
            }
        },
    ))
}
