//! Pigeonhole over a real slot fleet, through HTTP.

mod common;

use std::sync::Arc;
use std::time::Duration;

use autumn_web::reexports::{diesel, diesel_async};
use autumn_web::test::TestClient;
use diesel_async::RunQueryDsl as _;
use holes::{Flock, Slot, TenantName};
use pigeonhole_app::board::Board;
use pigeonhole_app::flock::{self, FlockState};
use pigeonhole_app::slot_of;

async fn hatch(client: &TestClient, name: &str) -> autumn_web::test::TestResponse {
    client
        .post("/pigeons")
        .header("hx-request", "true")
        .form(&format!("name={name}"))
        .send()
        .await
}

async fn write(client: &TestClient, who: &str, body: &str) {
    client
        .post("/pigeon/notes")
        .header("x-tenant-id", who)
        .form(&format!("body={body}"))
        .send()
        .await
        .assert_status(200);
}

#[tokio::test]
async fn hatching_registers_the_pigeon_and_creates_only_its_slot_file() {
    let tmp = tempfile::tempdir().unwrap();
    let fleet = common::fleet(tmp.path(), 4);
    let client = common::app(common::control_pool(tmp.path()).await, fleet.clone());
    let r = hatch(&client, "dusk-roller").await;
    r.assert_status(200);
    assert_eq!(r.header("hx-redirect"), Some("/pigeon"));
    let names: Vec<String> = fleet
        .list()
        .await
        .unwrap()
        .iter()
        .map(autumn_web::fleet_layout::FleetDbKey::name)
        .collect();
    assert_eq!(names, vec![slot_of("dusk-roller").db_name()]);
    client
        .get("/enter/dusk-roller")
        .send()
        .await
        .assert_status(303);
    client.get("/enter/nobody").send().await.assert_status(404);
    client.get("/enter/No.Body").send().await.assert_status(400);
}

/// The heart of slot mode: two tenants in one file, invisible to each other.
#[tokio::test]
async fn neighbours_share_a_file_and_never_see_each_other() {
    let tmp = tempfile::tempdir().unwrap();
    let fleet = common::fleet(tmp.path(), 4);
    let client = common::app(common::control_pool(tmp.path()).await, fleet.clone());
    let (a, b) = common::neighbours();
    assert_eq!(slot_of(&a), slot_of(&b));
    hatch(&client, &a).await.assert_status(200);
    hatch(&client, &b).await.assert_status(200);
    write(&client, &a, "only-for-a").await;
    write(&client, &b, "only-for-b").await;

    assert_eq!(fleet.list().await.unwrap().len(), 1, "one shared file");
    let mine = client
        .get("/pigeon/notes")
        .header("x-tenant-id", &a)
        .send()
        .await
        .text();
    assert!(mine.contains("only-for-a") && !mine.contains("only-for-b"));
    let theirs = client
        .get("/pigeon/notes")
        .header("x-tenant-id", &b)
        .send()
        .await
        .text();
    assert!(theirs.contains("only-for-b") && !theirs.contains("only-for-a"));

    // The page tells the truth about the file: 4 rows (2 hatch + 2 notes), 2 yours.
    let page = client.get("/pigeon").header("x-tenant-id", &a).send().await;
    page.assert_status(200)
        .assert_body_contains("<dt>rows in it</dt><dd>4</dd>")
        .assert_body_contains("<dt>yours</dt><dd>2</dd>")
        .assert_body_contains(&format!("/enter/{b}"));
}

#[tokio::test]
async fn a_flock_lands_every_note_in_the_right_file_and_the_census_agrees() {
    let tmp = tempfile::tempdir().unwrap();
    let fleet = common::fleet(tmp.path(), 6);
    let control = common::control_pool(tmp.path()).await;
    let client = common::app(control.clone(), fleet.clone());
    // Small enough for a default macOS `ulimit -n` of 256.
    let f = Flock::new(0, 400).unwrap();
    let state = Arc::new(FlockState::default());
    flock::run(&fleet, &control, &Arc::new(Board::new()), &state, f).await;
    let r = state.report();
    assert_eq!((r.failed, r.done), (0, 400), "{:?}", r.last_error);
    assert_eq!(r.notes, f.total_notes());

    let expected_files: std::collections::BTreeSet<Slot> =
        f.birds().map(|b| slot_of(b.name.as_str())).collect();
    assert_eq!(fleet.list().await.unwrap().len(), expected_files.len());

    let census = client.post("/census").send().await;
    census
        .assert_status(200)
        .assert_body_contains("Every row is in its pigeon")
        .assert_body_contains(&format!("{} files read", expected_files.len()));
}

/// Put a row in a file its tenant does not hash to; the census must name it.
#[tokio::test]
async fn the_census_finds_a_planted_stray() {
    let tmp = tempfile::tempdir().unwrap();
    let fleet = common::fleet(tmp.path(), 4);
    let client = common::app(common::control_pool(tmp.path()).await, fleet.clone());
    hatch(&client, "dusk-roller").await.assert_status(200);
    let home = slot_of("dusk-roller");
    let elsewhere = Slot::new(u32::from(home.get() ^ 1)).unwrap();
    let key = fleet.key_for_name(&elsewhere.db_name()).unwrap();
    let db = fleet.open(&key).await.unwrap();
    let mut conn = db.pool().get().await.unwrap();
    diesel::sql_query("INSERT INTO notes (body, tenant_id, created_at) VALUES ('lost', 'dusk-roller', '2026-10-31 00:00:00')")
        .execute(&mut *conn)
        .await
        .unwrap();
    drop(conn);
    let census = client.post("/census").send().await;
    census
        .assert_status(200)
        .assert_body_contains("The census disagrees")
        .assert_body_contains(&format!(
            "in <code>{}</code>, belongs in <code>{}</code>",
            elsewhere.db_name(),
            home.db_name()
        ));
}

/// FINDINGS 1: the census held the control connection through the whole
/// sweep. The log then charged the sweep to `SELECT ?`. Here the test holds
/// the only control connection. The sweep must start anyway.
#[tokio::test]
async fn the_census_sweeps_without_a_control_connection() {
    let tmp = tempfile::tempdir().unwrap();
    let fleet = common::fleet(tmp.path(), 4);
    let control = common::control_pool_sized(tmp.path(), 1).await;
    let client = common::app(control.clone(), fleet.clone());
    hatch(&client, "dusk-roller").await.assert_status(200);
    for key in fleet.open_keys() {
        fleet.close(&key).await;
    }
    assert_eq!(fleet.stats().open, 0, "every file is closed");
    let opened_before = fleet.stats().opens_total;

    let held = control.get().await.unwrap();
    let watch = async {
        let swept = tokio::time::timeout(Duration::from_secs(10), async {
            while fleet.stats().opens_total == opened_before {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .is_ok();
        drop(held);
        swept
    };
    let (census, swept) = tokio::join!(client.post("/census").send(), watch);
    assert!(
        swept,
        "the census waited for a control connection before the sweep"
    );
    census
        .assert_status(200)
        .assert_body_contains("Every row is in its pigeon");
}

#[tokio::test]
async fn the_board_draws_every_slot_and_the_slot_page_lists_its_pigeons() {
    let tmp = tempfile::tempdir().unwrap();
    let fleet = common::fleet(tmp.path(), 4);
    let client = common::app(common::control_pool(tmp.path()).await, fleet);
    let (a, b) = common::neighbours();
    hatch(&client, &a).await.assert_status(200);
    hatch(&client, &b).await.assert_status(200);
    let pic: serde_json::Value = client.get("/_board.json").send().await.json();
    let occ = pic["occupancy"].as_str().unwrap();
    assert_eq!(occ.len(), 16_384);
    let s = slot_of(&a);
    assert_eq!(&occ[usize::from(s.get())..=usize::from(s.get())], "2");
    assert_eq!(pic["pigeons"], 2);
    let page = client.get(&format!("/slot/{}", s.get())).send().await;
    page.assert_status(200)
        .assert_body_contains(&a)
        .assert_body_contains(&b);
    client.get("/slot/16384").send().await.assert_status(400);
}

#[test]
fn neighbour_names_are_valid_pigeons() {
    let (a, b) = common::neighbours();
    assert!(TenantName::parse(&a).is_ok() && TenantName::parse(&b).is_ok());
}
