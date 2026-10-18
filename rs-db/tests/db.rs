use rs_db::{Db, NewPlayer, PlayerSave};
use sqlx::PgPool;

fn player(name_hash: u64) -> NewPlayer<'static> {
    NewPlayer {
        name_hash,
        display_name: "Test",
        password_hash: "$argon2id$stub",
        staff_mod_level: 2,
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
