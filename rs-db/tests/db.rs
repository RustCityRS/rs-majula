use rs_db::{Db, NewPlayer, PlayerSave, WorldStatus};
use sqlx::PgPool;

fn player(name_hash: u64) -> NewPlayer<'static> {
    NewPlayer {
        name_hash,
        display_name: "Test",
        password_hash: "$argon2id$stub",
        staff_mod_level: 2,
        registration_ip: None,
    }
}

fn save() -> PlayerSave {
    PlayerSave {
        x: 3222,
        z: 3218,
        body: vec![0, 10, 18, 26, 33, 36, 42],
        colors: vec![1, 2, 3, 4, 5],
        stats: vec![1154; 21],
        levels: vec![10; 21],
        afk_zones: vec![7, 8],
        varp_ids: vec![281, 1000],
        varp_values: vec![1000, -1],
        inv_types: vec![93, 93, 94],
        inv_slots: vec![0, 27, 3],
        inv_objs: vec![995, 1351, 1277],
        inv_counts: vec![i32::MAX, 1, 1],
        ..Default::default()
    }
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn names_are_unique(pool: PgPool) {
    let db = Db::from_pool(pool);
    assert!(db.create_player(&player(42)).await.unwrap());
    assert!(!db.create_player(&player(42)).await.unwrap());
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn credentials_round_trip(pool: PgPool) {
    let db = Db::from_pool(pool.clone());
    assert!(db.find_credentials(42).await.unwrap().is_none());

    db.create_player(&player(42)).await.unwrap();
    let creds = db.find_credentials(42).await.unwrap().unwrap();
    assert_eq!(creds.password_hash, "$argon2id$stub");
    assert_eq!(creds.staff_mod_level, 2);
    assert!(!creds.banned);

    sqlx::query("UPDATE players SET banned_until = now() + interval '1 day'")
        .execute(&pool)
        .await
        .unwrap();
    assert!(db.find_credentials(42).await.unwrap().unwrap().banned);

    sqlx::query("UPDATE players SET banned_until = now() - interval '1 day'")
        .execute(&pool)
        .await
        .unwrap();
    assert!(!db.find_credentials(42).await.unwrap().unwrap().banned);
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn save_requires_a_player(pool: PgPool) {
    let db = Db::from_pool(pool);
    assert!(!db.store_save(42, &save()).await.unwrap());
    assert!(db.load_save(42).await.unwrap().is_none());
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn save_round_trips_and_overwrites(pool: PgPool) {
    let db = Db::from_pool(pool);
    db.create_player(&player(42)).await.unwrap();
    // Registered but never saved: the engine applies new-player defaults.
    assert!(db.load_save(42).await.unwrap().is_none());

    let first = save();
    assert!(db.store_save(42, &first).await.unwrap());
    assert_eq!(db.load_save(42).await.unwrap().unwrap(), first);

    let second = PlayerSave {
        x: 3094,
        playtime: 600,
        inv_types: vec![],
        inv_slots: vec![],
        inv_objs: vec![],
        inv_counts: vec![],
        ..first
    };
    assert!(db.store_save(42, &second).await.unwrap());
    assert_eq!(db.load_save(42).await.unwrap().unwrap(), second);
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn registrations_are_counted_per_ip(pool: PgPool) {
    let db = Db::from_pool(pool.clone());
    let ip: std::net::IpAddr = "203.0.113.7".parse().unwrap();
    let other: std::net::IpAddr = "2001:db8::1".parse().unwrap();

    for (hash, from) in [(1, Some(ip)), (2, Some(ip)), (3, Some(other)), (4, None)] {
        let new = NewPlayer {
            registration_ip: from,
            ..player(hash)
        };
        db.create_player(&new).await.unwrap();
    }
    assert!(db.name_taken(1).await.unwrap());
    assert!(!db.name_taken(99).await.unwrap());
    assert_eq!(db.recent_registrations(ip, 1).await.unwrap(), 2);
    assert_eq!(db.recent_registrations(other, 1).await.unwrap(), 1);

    // Registrations older than the window don't count.
    sqlx::query("UPDATE players SET created_at = now() - interval '2 hours' WHERE name_hash = 1")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(db.recent_registrations(ip, 1).await.unwrap(), 1);
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn news_pages_and_neighbours(pool: PgPool) {
    let db = Db::from_pool(pool.clone());
    assert_eq!(db.news_categories().await.unwrap().len(), 7);

    // Posts 1..=5 alternate categories 1 and 2.
    for i in 1..=5 {
        sqlx::query(
            "INSERT INTO news_posts (category_id, title, content, created_at)
             VALUES ($1, $2, '<b>hi</b>', '2004-03-01T12:00:00Z')",
        )
        .bind(if i % 2 == 1 { 1i16 } else { 2 })
        .bind(format!("post {i}"))
        .execute(&pool)
        .await
        .unwrap();
    }

    let all = db.news_page(None, 0, 3).await.unwrap();
    assert_eq!(all.iter().map(|p| p.id).collect::<Vec<_>>(), vec![5, 4, 3]);
    assert_eq!(all[0].short_date, "1-Mar-2004");
    assert_eq!(all[0].category_name, "Game Updates");

    let updates = db.news_page(Some(1), 1, 10).await.unwrap();
    assert_eq!(updates.iter().map(|p| p.id).collect::<Vec<_>>(), vec![3, 1]);

    let post = db.news_post(3).await.unwrap().unwrap();
    assert_eq!(post.long_date, "1st March 2004");
    assert_eq!((post.prev_id, post.next_id), (Some(1), Some(5)));
    let first = db.news_post(1).await.unwrap().unwrap();
    assert_eq!((first.prev_id, first.next_id), (None, Some(3)));
    assert!(db.news_post(99).await.unwrap().is_none());
}

#[sqlx::test(migrations = false)]
#[ignore = "needs DATABASE_URL"]
async fn schema_check_tracks_this_build(pool: PgPool) {
    let db = Db::from_pool(pool.clone());
    let all: Vec<i64> = rs_db::MIGRATOR.iter().map(|m| m.version).collect();
    assert_eq!(db.pending_migrations().await.unwrap(), all);

    db.migrate().await.unwrap();
    assert!(db.pending_migrations().await.unwrap().is_empty());

    // A newer build's migration doesn't make this one stale.
    sqlx::query(
        "INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time)
         VALUES (99999, 'future', true, decode('00', 'hex'), 0)",
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(db.pending_migrations().await.unwrap().is_empty());

    sqlx::query("UPDATE _sqlx_migrations SET checksum = decode('00', 'hex') WHERE version = $1")
        .bind(all[0])
        .execute(&pool)
        .await
        .unwrap();
    assert!(db.pending_migrations().await.is_err());
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn world_status_goes_stale(pool: PgPool) {
    let db = Db::from_pool(pool.clone());
    db.report_world_status(10, 3).await.unwrap();
    db.report_world_status(11, 0).await.unwrap();
    db.report_world_status(10, 5).await.unwrap();
    sqlx::query(
        "UPDATE world_status SET updated_at = now() - interval '1 minute' WHERE node_id = 11",
    )
    .execute(&pool)
    .await
    .unwrap();

    let live = db.live_worlds(30).await.unwrap();
    assert_eq!(
        live,
        vec![WorldStatus {
            node_id: 10,
            players: 5
        }]
    );
}
