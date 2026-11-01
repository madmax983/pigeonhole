// Pigeonhole — sixteen thousand holes, more pigeons than that.
//
//   Engine       -> holes: tenant names, the 128 × 128 slot board, the census
//                   fold, the flock plan. No IO, no async, no clock.
//   Fleet        -> autumn-web 0.9's DatabaseFleet in slot mode: at most
//                   16,384 SQLite files under `pigeonhole-fleet/`, one per
//                   routing slot, each shared by every tenant that hashes there.
//   Control      -> one SQLite file for sessions and the pigeon registry.
//   Live events  -> one SSE topic; the board redraws on it.
//
// Run with:  cargo run -p pigeonhole-app
// Then open: http://localhost:3000

use autumn_web::migrate::{EmbeddedMigrations, embed_migrations};
use autumn_web::prelude::*;
use pigeonhole_app::routes;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

#[autumn_web::main]
async fn main() {
    // Boxed: the server future is too big to keep on the stack.
    Box::pin(
        autumn_web::app()
            // One set, applied to the control database AND to every slot file:
            // autumn-web 0.9 has no control-only app set. See
            // docs/adr/0002-one-migration-set.md.
            .migrations(MIGRATIONS)
            .on_startup(pigeonhole_app::boot)
            .routes(routes![
                routes::board::index,
                routes::board::picture,
                routes::board::panel,
                routes::board::stream,
                routes::board::found,
                routes::board::enter,
                routes::board::release,
                routes::board::slot,
                routes::pigeon::index,
                routes::pigeon::notes,
                routes::pigeon::add_note,
                routes::census::index,
                routes::census::take,
                routes::about::show,
                routes::assets::script,
            ])
            .run(),
    )
    .await;
}
