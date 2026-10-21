use rs_db::{
    AppealFiled, ChatLog, Claim, Db, LogBatch, LoginDevice, ModAction, NewAppeal, NewPlayer,
    NewReplay, NewSnapshot, PlayerSave, RESET_LOCKED, Reconciled, ReplayChunk, ReplayReport,
    Sanctions, SavePoint, SaveResult, SessionLog, TimelineCursor, WealthEvent, WorldStatus,
    transfer,
};
use sqlx::PgPool;

fn claimed(save: Option<PlayerSave>) -> Claim {
    Claim::Claimed {
        save: save.map(Box::new),
        took_over: None,
    }
}

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
    assert!(db.find_credentials(42, 1).await.unwrap().is_none());

    db.create_player(&player(42)).await.unwrap();
    let creds = db.find_credentials(42, 1).await.unwrap().unwrap();
    assert_eq!(creds.password_hash, "$argon2id$stub");
    assert_eq!(creds.staff_mod_level, 2);
    assert!(!creds.banned);

    sqlx::query("UPDATE players SET banned_until = now() + interval '1 day'")
        .execute(&pool)
        .await
        .unwrap();
    assert!(db.find_credentials(42, 1).await.unwrap().unwrap().banned);

    sqlx::query("UPDATE players SET banned_until = now() - interval '1 day'")
        .execute(&pool)
        .await
        .unwrap();
    assert!(!db.find_credentials(42, 1).await.unwrap().unwrap().banned);
}

async fn muted_ms(db: &Db, name_hash: u64) -> Option<i64> {
    let creds = db.find_credentials(name_hash, 1).await.unwrap();
    creds.unwrap().muted_ms
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn mutes_respect_rank_and_expire(pool: PgPool) {
    let db = Db::from_pool(pool.clone());
    // `player()` is a J-Mod; 7 is a regular player.
    db.create_player(&player(42)).await.unwrap();
    db.create_player(&NewPlayer {
        staff_mod_level: 0,
        ..player(7)
    })
    .await
    .unwrap();
    assert_eq!(muted_ms(&db, 7).await, None);
    assert_eq!(db.sanctions(&[7, 42, 99]).await.unwrap().len(), 2);

    // A P-Mod can't mute a J-Mod, or an unknown name.
    assert_eq!(db.mute_player(42, 48, 1).await.unwrap(), None);
    assert_eq!(db.mute_player(99, 48, 1).await.unwrap(), None);

    let hours = |ms: i64| (ms + 60_000) / 3_600_000;
    let ms = db.mute_player(7, 48, 1).await.unwrap().unwrap();
    assert_eq!(hours(ms), 48);
    assert_eq!(hours(muted_ms(&db, 7).await.unwrap()), 48);

    // A shorter mute doesn't cut a longer one short.
    let ms = db.mute_player(7, 1, 1).await.unwrap().unwrap();
    assert_eq!(hours(ms), 48);

    let mut mutes: Vec<_> = db
        .sanctions(&[7, 42])
        .await
        .unwrap()
        .into_iter()
        .map(|s| (s.name_hash, s.muted_ms.is_some()))
        .collect();
    mutes.sort();
    assert_eq!(mutes, [(7, true), (42, false)]);

    sqlx::query("UPDATE players SET muted_until = now() - interval '1 day'")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(muted_ms(&db, 7).await, None);
    assert_eq!(
        db.sanctions(&[7]).await.unwrap(),
        vec![Sanctions {
            name_hash: 7,
            muted_ms: None,
            banned: false
        }]
    );
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn moderation_respects_rank_and_is_logged(pool: PgPool) {
    let db = Db::from_pool(pool.clone());
    // `player()` is a J-Mod; 7 is a regular player.
    db.create_player(&player(42)).await.unwrap();
    db.create_player(&NewPlayer {
        staff_mod_level: 0,
        ..player(7)
    })
    .await
    .unwrap();
    let state = |db: Db| async move {
        let s = db.sanctions(&[7]).await.unwrap().remove(0);
        (s.muted_ms.map(|ms| (ms + 60_000) / 3_600_000), s.banned)
    };

    // A J-Mod can't act on a J-Mod, an unknown name, or themselves.
    assert!(
        !db.moderate(42, 2, 42, ModAction::Kick, None, 10)
            .await
            .unwrap()
    );
    assert!(
        !db.moderate(42, 2, 99, ModAction::Unban, None, 10)
            .await
            .unwrap()
    );

    assert!(
        db.moderate(42, 2, 7, ModAction::Mute { hours: 48 }, None, 10)
            .await
            .unwrap()
    );
    assert_eq!(state(db.clone()).await, (Some(48), false));
    // A command sets the length outright, shorter included.
    assert!(
        db.moderate(42, 2, 7, ModAction::Mute { hours: 2 }, None, 10)
            .await
            .unwrap()
    );
    assert_eq!(state(db.clone()).await, (Some(2), false));
    assert!(
        db.moderate(42, 2, 7, ModAction::Unmute, None, 10)
            .await
            .unwrap()
    );
    assert!(
        db.moderate(42, 2, 7, ModAction::Ban { hours: 24 }, None, 10)
            .await
            .unwrap()
    );
    assert_eq!(state(db.clone()).await, (None, true));
    assert!(db.find_credentials(7, 1).await.unwrap().unwrap().banned);
    assert!(
        db.moderate(42, 2, 7, ModAction::Unban, None, 10)
            .await
            .unwrap()
    );
    assert_eq!(state(db.clone()).await, (None, false));
    assert!(
        db.moderate(42, 2, 7, ModAction::Kick, None, 10)
            .await
            .unwrap()
    );

    let logged: Vec<(String, Option<i32>, i16)> = sqlx::query_as(
        "SELECT l.action, l.hours, l.node_id FROM moderation_logs l
         JOIN players m ON m.id = l.moderator_id AND m.name_hash = 42
         JOIN players t ON t.id = l.target_id AND t.name_hash = 7
         ORDER BY l.id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    let row = |action: &str, hours| (action.to_owned(), hours, 10);
    assert_eq!(
        logged,
        vec![
            row("mute", Some(48)),
            row("mute", Some(2)),
            row("unmute", None),
            row("ban", Some(24)),
            row("unban", None),
            row("kick", None),
        ]
    );
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn bans_keep_the_longest(pool: PgPool) {
    let db = Db::from_pool(pool.clone());
    assert!(!db.ban_player(42, 48).await.unwrap());
    db.create_player(&player(42)).await.unwrap();

    assert!(db.ban_player(42, 48).await.unwrap());
    assert!(db.find_credentials(42, 1).await.unwrap().unwrap().banned);
    assert!(db.ban_player(42, 1).await.unwrap());
    let hours: f64 = sqlx::query_scalar(
        "SELECT extract(epoch FROM banned_until - now())::float8 / 3600 FROM players",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(hours > 47.0);
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn snapshots_need_a_registered_reporter(pool: PgPool) {
    let db = Db::from_pool(pool.clone());
    let snapshot = NewSnapshot {
        reporter_name_hash: 42,
        // Need not be registered.
        offender_name_hash: 99,
        reason: 11,
        node_id: 10,
        x: 3222,
        z: 3218,
        y: 0,
        muted: true,
    };
    assert!(!db.record_snapshot(&snapshot).await.unwrap());

    db.create_player(&player(42)).await.unwrap();
    assert!(db.record_snapshot(&snapshot).await.unwrap());
    let row: (i64, i16, i16, i16, bool) =
        sqlx::query_as("SELECT offender_name_hash, reason, node_id, x, muted FROM snapshots")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(row, (99, 11, 10, 3222, true));
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn abuse_reports_page_newest_first(pool: PgPool) {
    let db = Db::from_pool(pool);
    db.create_player(&player(42)).await.unwrap();
    db.create_player(&player(43)).await.unwrap();
    for (offender, reason) in [(99, 6), (43, 0), (99, 11)] {
        let snapshot = NewSnapshot {
            reporter_name_hash: 42,
            offender_name_hash: offender,
            reason,
            node_id: 10,
            x: 3222,
            z: 3218,
            y: 0,
            muted: false,
        };
        assert!(db.record_snapshot(&snapshot).await.unwrap());
    }

    let all = db.abuse_reports(None, None, 10).await.unwrap();
    assert_eq!(all.iter().map(|r| r.reason).collect::<Vec<_>>(), [11, 0, 6]);
    assert_eq!(all[0].reporter, "Test");
    // Only registered offenders have a name on record.
    assert_eq!(all[0].offender, None);
    assert_eq!(all[1].offender.as_deref(), Some("Test"));

    let against = db.abuse_reports(Some(99), None, 10).await.unwrap();
    assert_eq!(against.len(), 2);
    let older = db
        .abuse_reports(Some(99), Some(against[0].id), 10)
        .await
        .unwrap();
    assert_eq!(older.len(), 1);
    assert_eq!(older[0].reason, 6);
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn logs_are_recorded_in_order(pool: PgPool) {
    let db = Db::from_pool(pool.clone());
    db.create_player(&player(42)).await.unwrap();
    let chat = |name_hash, to_name_hash, message: &str| ChatLog {
        name_hash,
        to_name_hash,
        x: 3222,
        z: 3218,
        y: 0,
        message: message.to_owned(),
    };
    let session = |name_hash, event_type, event: &str| SessionLog {
        name_hash,
        x: 3222,
        z: 3218,
        y: 0,
        event_type,
        event: event.to_owned(),
    };
    let wealth = |name_hash, item: &str, count, value| WealthEvent {
        name_hash,
        x: 3222,
        z: 3218,
        y: 0,
        event_type: 6,
        item: item.to_owned(),
        count,
        value,
        counterpart: None,
    };
    db.record_logs(
        10,
        &LogBatch {
            chat: vec![
                chat(42, None, "hello"),
                // Not registered: skipped.
                chat(99, None, "lost"),
                chat(42, Some(99), "psst"),
            ],
            sessions: vec![
                session(42, 1, "Quest complete: Clock Tower"),
                session(99, 0, "lost"),
                session(42, 0, "Player died"),
            ],
            wealth: vec![wealth(42, "bronze_axe", 1, 16), wealth(99, "lost", 1, 1)],
        },
    )
    .await
    .unwrap();
    db.record_logs(10, &LogBatch::default()).await.unwrap();

    let rows: Vec<(Option<i64>, i16, String)> =
        sqlx::query_as("SELECT to_name_hash, node_id, message FROM chat_logs ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(
        rows,
        vec![
            (None, 10, "hello".to_owned()),
            (Some(99), 10, "psst".to_owned()),
        ]
    );
    let rows: Vec<(i16, String)> =
        sqlx::query_as("SELECT event_type, event FROM session_logs ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(
        rows,
        vec![
            (1, "Quest complete: Clock Tower".to_owned()),
            (0, "Player died".to_owned()),
        ]
    );
    let rows: Vec<(i16, i16, String, i32, i32)> =
        sqlx::query_as("SELECT node_id, event_type, item, count, value FROM wealth_events")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(rows, vec![(10, 6, "bronze_axe".to_owned(), 1, 16)]);
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn logout_is_reported_to_other_worlds(pool: PgPool) {
    let db = Db::from_pool(pool.clone());
    db.create_player(&player(42)).await.unwrap();
    let left = |node| {
        let db = db.clone();
        async move {
            db.find_credentials(42, node)
                .await
                .unwrap()
                .unwrap()
                .left_other_world_ms
        }
    };
    // Never logged out.
    assert_eq!(left(1).await, None);

    db.claim_session(42, 1, 60).await.unwrap();
    assert_eq!(
        db.store_logout_save(42, 1, &save()).await.unwrap(),
        SaveResult::Stored
    );
    assert_eq!(db.load_save(42).await.unwrap().unwrap(), save());
    // The same world doesn't count; another world sees a fresh logout.
    assert_eq!(left(1).await, None);
    assert!(left(2).await.unwrap() < 5_000);

    sqlx::query("UPDATE players SET last_logout_at = now() - interval '1 minute'")
        .execute(&pool)
        .await
        .unwrap();
    assert!(left(2).await.unwrap() >= 60_000);
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn save_requires_a_player(pool: PgPool) {
    let db = Db::from_pool(pool);
    assert_eq!(
        db.store_save(42, 1, &save()).await.unwrap(),
        SaveResult::Unregistered
    );
    assert_eq!(
        db.store_logout_save(42, 1, &save()).await.unwrap(),
        SaveResult::Unregistered
    );
    assert_eq!(
        db.claim_session(42, 1, 60).await.unwrap(),
        Claim::Unregistered
    );
    assert!(db.load_save(42).await.unwrap().is_none());
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn save_round_trips_and_overwrites(pool: PgPool) {
    let db = Db::from_pool(pool);
    db.create_player(&player(42)).await.unwrap();
    // Registered but never saved: the engine applies new-player defaults.
    assert_eq!(db.claim_session(42, 1, 60).await.unwrap(), claimed(None));

    let first = save();
    assert_eq!(
        db.store_save(42, 1, &first).await.unwrap(),
        SaveResult::Stored
    );
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
    assert_eq!(
        db.store_save(42, 1, &second).await.unwrap(),
        SaveResult::Stored
    );
    assert_eq!(db.load_save(42).await.unwrap().unwrap(), second);
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn only_the_owning_world_saves(pool: PgPool) {
    let db = Db::from_pool(pool);
    db.create_player(&player(42)).await.unwrap();
    // Nobody owns the session: every save is refused.
    assert_eq!(
        db.store_save(42, 1, &save()).await.unwrap(),
        SaveResult::NotOwner
    );

    db.claim_session(42, 1, 60).await.unwrap();
    // World 1 is alive (fresh heartbeat), so world 2 can't claim or save.
    db.report_world_status(1, 1, false).await.unwrap();
    assert_eq!(
        db.claim_session(42, 2, 60).await.unwrap(),
        Claim::Busy { node: 1 }
    );
    assert_eq!(
        db.store_save(42, 2, &save()).await.unwrap(),
        SaveResult::NotOwner
    );
    assert_eq!(
        db.store_logout_save(42, 2, &save()).await.unwrap(),
        SaveResult::NotOwner
    );
    assert!(db.load_save(42).await.unwrap().is_none());

    // The logout save ends the session; world 2 then claims and sees it.
    assert_eq!(
        db.store_logout_save(42, 1, &save()).await.unwrap(),
        SaveResult::Stored
    );
    assert_eq!(
        db.store_save(42, 1, &save()).await.unwrap(),
        SaveResult::NotOwner
    );
    assert_eq!(
        db.claim_session(42, 2, 60).await.unwrap(),
        claimed(Some(save()))
    );
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn dead_worlds_lose_their_sessions(pool: PgPool) {
    let db = Db::from_pool(pool.clone());
    db.create_player(&player(42)).await.unwrap();
    db.claim_session(42, 1, 60).await.unwrap();
    db.report_world_status(1, 1, false).await.unwrap();

    sqlx::query("UPDATE world_status SET updated_at = now() - interval '2 minutes'")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        db.claim_session(42, 2, 60).await.unwrap(),
        Claim::Claimed {
            save: None,
            took_over: Some(1)
        }
    );
    // The dead world's late saves are refused.
    assert_eq!(
        db.store_save(42, 1, &save()).await.unwrap(),
        SaveResult::NotOwner
    );
    assert_eq!(
        db.store_save(42, 2, &save()).await.unwrap(),
        SaveResult::Stored
    );
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn sessions_are_released_and_reconciled(pool: PgPool) {
    let db = Db::from_pool(pool);
    for hash in [1, 2, 3, 4] {
        db.create_player(&player(hash)).await.unwrap();
        db.claim_session(hash, 1, 60).await.unwrap();
    }
    db.report_world_status(1, 1, false).await.unwrap();

    assert!(db.release_session(4, 1).await.unwrap());
    assert!(!db.release_session(4, 1).await.unwrap());
    // Only the owner can release.
    assert!(!db.release_session(3, 2).await.unwrap());

    // 1 is online, 2 is mid-login, 3 was leaked; 4 was taken by world 2.
    db.claim_session(4, 2, 60).await.unwrap();
    let fixed = db.reconcile_sessions(1, &[1, 4], &[2]).await.unwrap();
    assert_eq!(
        fixed,
        Reconciled {
            released: vec![3],
            lost: vec![4],
        }
    );
    assert_eq!(
        db.store_save(1, 1, &save()).await.unwrap(),
        SaveResult::Stored
    );
    assert_eq!(
        db.store_save(3, 1, &save()).await.unwrap(),
        SaveResult::NotOwner
    );

    // A restart ends everything the world still owned, as a logout, except
    // players with data still on its disk.
    assert_eq!(db.release_node_sessions(1, &[2]).await.unwrap(), 1);
    assert_eq!(
        db.store_save(2, 1, &save()).await.unwrap(),
        SaveResult::Stored
    );
    assert_eq!(db.release_node_sessions(1, &[]).await.unwrap(), 1);
    assert_eq!(
        db.store_save(1, 1, &save()).await.unwrap(),
        SaveResult::NotOwner
    );
    let creds = db.find_credentials(1, 2).await.unwrap().unwrap();
    assert!(creds.left_other_world_ms.is_some());
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn imports_only_fill_an_empty_offline_player(pool: PgPool) {
    let db = Db::from_pool(pool);
    assert!(!db.import_save(42, &save()).await.unwrap());
    db.create_player(&player(42)).await.unwrap();
    assert!(db.import_save(42, &save()).await.unwrap());
    // Never over an existing save.
    assert!(!db.import_save(42, &PlayerSave::default()).await.unwrap());
    assert_eq!(db.load_save(42).await.unwrap().unwrap(), save());

    db.create_player(&player(43)).await.unwrap();
    db.claim_session(43, 1, 60).await.unwrap();
    assert!(!db.import_save(43, &save()).await.unwrap());
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn login_failures_are_counted_per_ip(pool: PgPool) {
    let db = Db::from_pool(pool.clone());
    let ip: std::net::IpAddr = "203.0.113.7".parse().unwrap();
    let other: std::net::IpAddr = "2001:db8::1".parse().unwrap();
    for _ in 0..3 {
        db.record_login_failure(ip).await.unwrap();
    }
    db.record_login_failure(other).await.unwrap();
    assert_eq!(db.recent_login_failures(ip, 60).await.unwrap(), 3);
    assert_eq!(db.recent_login_failures(other, 60).await.unwrap(), 1);

    sqlx::query(
        "UPDATE login_failures SET at = now() - interval '2 minutes'
         WHERE ip = '203.0.113.7'::inet",
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(db.recent_login_failures(ip, 60).await.unwrap(), 0);
    db.prune_login_failures(60).await.unwrap();
    let left: i64 = sqlx::query_scalar("SELECT count(*) FROM login_failures")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(left, 1);
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
    db.report_world_status(10, 3, false).await.unwrap();
    db.report_world_status(11, 0, false).await.unwrap();
    // The latest heartbeat says whether it's in maintenance.
    db.report_world_status(10, 5, true).await.unwrap();
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
            players: 5,
            maintenance: true,
        }]
    );

    // A touch renews the heartbeat but keeps the player count.
    db.touch_world(11).await.unwrap();
    db.touch_world(12).await.unwrap();
    let mut live = db.live_worlds(30).await.unwrap();
    live.sort_by_key(|w| w.node_id);
    assert_eq!(
        live.iter()
            .map(|w| (w.node_id, w.players))
            .collect::<Vec<_>>(),
        vec![(10, 5), (11, 0), (12, 0)]
    );

    db.retire_world(10).await.unwrap();
    assert!(
        !db.live_worlds(30)
            .await
            .unwrap()
            .iter()
            .any(|w| w.node_id == 10)
    );
}

/// Registers a player and logs them in and out with these stats (XP x10,
/// indexed by stat id).
async fn ranked(db: &Db, name_hash: u64, name: &'static str, staff: i16, stats: &[(usize, i32)]) {
    db.create_player(&NewPlayer {
        display_name: name,
        staff_mod_level: staff,
        ..player(name_hash)
    })
    .await
    .unwrap();
    resave(db, name_hash, stats).await;
}

fn stats_save(stats: &[(usize, i32)]) -> PlayerSave {
    let mut all = vec![0; 21];
    for &(stat, xp) in stats {
        all[stat] = xp;
    }
    PlayerSave {
        stats: all,
        ..save()
    }
}

/// Logs a player in and out with these stats.
async fn resave(db: &Db, name_hash: u64, stats: &[(usize, i32)]) {
    db.claim_session(name_hash, 1, 60).await.unwrap();
    assert_eq!(
        db.store_logout_save(name_hash, 1, &stats_save(stats))
            .await
            .unwrap(),
        SaveResult::Stored
    );
}

fn names(page: &[rs_db::HiscoreEntry]) -> Vec<&str> {
    page.iter().map(|e| e.name.as_str()).collect()
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn hiscores_rank_saved_stats(pool: PgPool) {
    let db = Db::from_pool(pool.clone());
    // Attack 99 (13,034,431 XP) + Hitpoints 10.
    ranked(&db, 1, "Maxed", 0, &[(0, 130_344_310), (3, 11_540)]).await;
    // Attack 40 + Runecrafting 2.
    ranked(&db, 2, "Middle", 0, &[(0, 372_240), (3, 11_540), (20, 830)]).await;
    // Same Attack as Middle, reached later.
    ranked(&db, 3, "Second", 0, &[(0, 372_240), (3, 11_540)]).await;
    // Never ranked: a J-Mod, and a banned player.
    ranked(&db, 4, "Jmod", 2, &[(0, 200_000_000)]).await;
    ranked(&db, 5, "Banned", 0, &[(0, 200_000_000)]).await;
    sqlx::query("UPDATE players SET banned_until = now() + interval '1 day' WHERE name_hash = 5")
        .execute(&pool)
        .await
        .unwrap();

    let attack = db.hiscores_page(1, 0, 10).await.unwrap();
    assert_eq!(names(&attack), ["Maxed", "Middle", "Second"]);
    assert_eq!(
        (attack[0].rank, attack[0].level, attack[0].xp),
        (1, 99, 13_034_431)
    );
    assert_eq!(db.hiscores_count(1).await.unwrap(), 3);

    // Overall: total level over the 19 skills (untrained ones count as 1).
    let overall = db.hiscores_page(0, 0, 10).await.unwrap();
    assert_eq!(names(&overall), ["Maxed", "Middle", "Second"]);
    assert_eq!(overall[0].level, 99 + 10 + 17);
    assert_eq!(overall[1].level, 40 + 10 + 2 + 16);

    // Untrained skills aren't ranked.
    assert_eq!(
        names(&db.hiscores_page(21, 0, 10).await.unwrap()),
        ["Middle"]
    );
    let second = db.player_hiscores(3).await.unwrap().unwrap();
    assert_eq!(second.name, "Second");
    assert_eq!(second.rank(1).unwrap().rank, 3);
    assert!(second.rank(21).is_none());

    // Saving unchanged XP keeps your place; gaining XP moves you.
    resave(&db, 2, &[(0, 372_240), (3, 11_540), (20, 830)]).await;
    assert_eq!(
        db.player_hiscores(2)
            .await
            .unwrap()
            .unwrap()
            .rank(1)
            .unwrap()
            .rank,
        2
    );
    resave(&db, 3, &[(0, 372_241), (3, 11_540)]).await;
    assert_eq!(
        db.player_hiscores(3)
            .await
            .unwrap()
            .unwrap()
            .rank(1)
            .unwrap()
            .rank,
        2
    );

    // An autosave doesn't touch the hiscores; the logout save does.
    db.claim_session(2, 1, 60).await.unwrap();
    let more = stats_save(&[(0, 130_344_310), (3, 11_540), (20, 830)]);
    assert_eq!(
        db.store_save(2, 1, &more).await.unwrap(),
        SaveResult::Stored
    );
    let middle = db.player_hiscores(2).await.unwrap().unwrap();
    assert_eq!(middle.rank(1).unwrap().rank, 3);
    assert_eq!(
        db.store_logout_save(2, 1, &more).await.unwrap(),
        SaveResult::Stored
    );
    let middle = db.player_hiscores(2).await.unwrap().unwrap();
    assert_eq!(middle.rank(1).unwrap().rank, 2);

    assert!(db.player_hiscores(4).await.unwrap().is_none());
    assert!(db.player_hiscores(5).await.unwrap().is_none());
    assert!(db.player_hiscores(99).await.unwrap().is_none());

    // Paging carries the absolute rank.
    let tail = db.hiscores_page(1, 1, 10).await.unwrap();
    assert_eq!(tail.iter().map(|e| e.rank).collect::<Vec<_>>(), [2, 3]);
}

/// The migration's SQL backfill builds the same rows a save does.
#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn hiscores_backfill_matches_saves(pool: PgPool) {
    let db = Db::from_pool(pool.clone());
    ranked(
        &db,
        1,
        "Maxed",
        0,
        &[(0, 130_344_310), (3, 11_540), (20, 2_000_000_000)],
    )
    .await;
    ranked(&db, 2, "Fresh", 0, &[(3, 11_540)]).await;
    ranked(
        &db,
        3,
        "Odd",
        0,
        &[(0, 830), (17, 99_999), (18, 5_000), (19, 5_000)],
    )
    .await;

    let rows = "SELECT player_id, category, level, xp FROM hiscores ORDER BY 1, 2";
    let saved: Vec<(i64, i16, i16, i64)> = sqlx::query_as(rows).fetch_all(&pool).await.unwrap();
    assert_eq!(saved.len(), 3 * 20);

    let migration = include_str!("../migrations/0005_hiscores.sql");
    let backfill = &migration[migration.find("WITH skills").unwrap()..];
    sqlx::query("DELETE FROM hiscores")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::raw_sql(backfill).execute(&pool).await.unwrap();
    let backfilled: Vec<(i64, i16, i16, i64)> =
        sqlx::query_as(rows).fetch_all(&pool).await.unwrap();
    assert_eq!(backfilled, saved);
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn replays_store_list_and_prune(pool: PgPool) {
    let db = Db::from_pool(pool.clone());
    let replay = |id, name_hash| NewReplay {
        id,
        name_hash,
        node_id: 10,
        revision: "289".into(),
    };
    // Only registered players are recorded.
    assert!(!db.start_replay(&replay(1, 42)).await.unwrap());
    db.create_player(&player(42)).await.unwrap();
    assert!(db.start_replay(&replay(1, 42)).await.unwrap());
    assert!(db.start_replay(&replay(2, 42)).await.unwrap());

    let chunk = |seq, data: &[u8]| ReplayChunk {
        seq,
        start_ms: seq * 30_000,
        end_ms: (seq + 1) * 30_000,
        data: data.to_vec(),
    };
    let clean = ReplayReport::default();
    db.store_replay(1, Some(&chunk(0, b"abc")), 30_000, &clean, false, true)
        .await
        .unwrap();
    let flagged = ReplayReport {
        score: 40,
        flags: vec!["no_heartbeat".into()],
        stats: "{}".into(),
    };
    db.store_replay(1, Some(&chunk(1, b"defg")), 41_000, &flagged, true, true)
        .await
        .unwrap();
    // A chunk for a recording that isn't there is dropped.
    db.store_replay(99, Some(&chunk(0, b"x")), 1, &clean, true, true)
        .await
        .unwrap();

    let one = db.replay(1).await.unwrap().unwrap();
    assert_eq!(one.player, "Test");
    assert_eq!((one.duration_ms, one.bytes, one.score), (41_000, 7, 40));
    assert_eq!(one.flags, ["no_heartbeat"]);
    assert!(one.ended_ms.is_some() && one.complete);
    let chunks = db.replay_chunks(1).await.unwrap();
    assert_eq!(
        chunks.iter().map(|c| c.data.as_slice()).collect::<Vec<_>>(),
        [b"abc".as_slice(), b"defg"]
    );

    assert_eq!(
        db.replays(Some(42), false, None, None, 10)
            .await
            .unwrap()
            .len(),
        2
    );
    let only_flagged = db.replays(None, true, None, None, 10).await.unwrap();
    assert_eq!(only_flagged.iter().map(|r| r.id).collect::<Vec<_>>(), [1]);

    sqlx::query("UPDATE replays SET started_at = now() - interval '8 days' WHERE id = 1")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(db.prune_replays(7).await.unwrap(), 1);
    assert!(db.replay(1).await.unwrap().is_none());
    assert!(db.replay_chunks(1).await.unwrap().is_empty());
    assert!(db.replay(2).await.unwrap().is_some());
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn moderation_reasons_are_logged(pool: PgPool) {
    let db = Db::from_pool(pool.clone());
    db.create_player(&player(42)).await.unwrap();
    db.create_player(&NewPlayer {
        staff_mod_level: 0,
        ..player(7)
    })
    .await
    .unwrap();
    let ban = ModAction::Ban { hours: 48 };
    assert!(
        db.moderate(42, 2, 7, ban, Some(" Macroing (offence 1) "), 10)
            .await
            .unwrap()
    );
    assert!(
        db.moderate(42, 2, 7, ModAction::Unban, Some("  "), 10)
            .await
            .unwrap()
    );
    let history = db
        .moderation_history(Some(7), None, None, 10)
        .await
        .unwrap();
    let reasons: Vec<Option<&str>> = history.iter().map(|m| m.reason.as_deref()).collect();
    assert_eq!(reasons, [None, Some("Macroing (offence 1)")]);
    let by = |moderator| db.moderation_history(None, Some(moderator), None, 10);
    assert_eq!(by(42).await.unwrap().len(), 2);
    assert!(by(7).await.unwrap().is_empty());
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn logins_link_accounts_by_address(pool: PgPool) {
    let db = Db::from_pool(pool.clone());
    let a: std::net::IpAddr = "203.0.113.7".parse().unwrap();
    let b: std::net::IpAddr = "2001:db8::1".parse().unwrap();
    let c: std::net::IpAddr = "198.51.100.2".parse().unwrap();
    // 1 registered from a; 4 registered from a but never logged in.
    for (hash, from) in [(1, Some(a)), (2, None), (3, None), (4, Some(a)), (5, None)] {
        db.create_player(&NewPlayer {
            staff_mod_level: 0,
            registration_ip: from,
            ..player(hash)
        })
        .await
        .unwrap();
    }
    for (hash, ip) in [(1, a), (1, a), (2, a), (2, b), (3, b), (5, c), (99, c)] {
        db.record_login(hash, 10, ip, None, &LoginDevice::default())
            .await
            .unwrap();
    }

    let addresses = db.login_addresses(1, 10).await.unwrap();
    assert_eq!(addresses.len(), 1);
    assert_eq!(
        (addresses[0].ip.as_str(), addresses[0].logins),
        ("203.0.113.7", 2)
    );
    // 2 logged in from it and 4 registered from it.
    assert_eq!(addresses[0].others, 2);

    let names = |links: Vec<rs_db::LinkedAccount>| {
        let mut names: Vec<(i64, Vec<String>)> =
            links.into_iter().map(|l| (l.logins, l.shared)).collect();
        names.sort();
        names
    };
    // 1 shares a with 2 (a login) and 4 (a registration); not 3, who only
    // shares b with 2.
    assert_eq!(
        names(db.linked_accounts(1, 10).await.unwrap()),
        vec![
            (0, vec!["203.0.113.7".to_owned()]),
            (1, vec!["203.0.113.7".to_owned()]),
        ]
    );
    assert_eq!(db.linked_accounts(2, 10).await.unwrap().len(), 3);
    assert!(db.linked_accounts(5, 10).await.unwrap().is_empty());

    // Bans show on the linked account.
    sqlx::query("UPDATE players SET banned_until = now() + interval '1 day' WHERE name_hash = 3")
        .execute(&pool)
        .await
        .unwrap();
    let banned = db.linked_accounts(2, 10).await.unwrap();
    assert_eq!(
        banned
            .iter()
            .filter(|l| l.banned_until_ms.is_some())
            .count(),
        1
    );

    // Old logins are pruned.
    sqlx::query(
        "UPDATE login_logs SET created_at = now() - interval '200 days' WHERE ip = $1::inet",
    )
    .bind(b.to_string())
    .execute(&pool)
    .await
    .unwrap();
    db.prune_login_logs(180).await.unwrap();
    assert!(db.login_addresses(3, 10).await.unwrap().is_empty());
    assert_eq!(db.login_addresses(1, 10).await.unwrap().len(), 1);
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn logins_link_accounts_by_client(pool: PgPool) {
    let db = Db::from_pool(pool);
    let home: std::net::IpAddr = "203.0.113.7".parse().unwrap();
    let vpn: std::net::IpAddr = "198.51.100.2".parse().unwrap();
    let cafe: std::net::IpAddr = "192.0.2.9".parse().unwrap();
    const NAMES: [&str; 4] = ["One", "Two", "Three", "Four"];
    for (hash, name) in (1..).zip(NAMES) {
        db.create_player(&NewPlayer {
            display_name: name,
            ..player(hash)
        })
        .await
        .unwrap();
    }
    // 2 shares 1's install from another address; 3 shares both; 4 nothing.
    for (hash, ip, uid) in [
        (1, home, Some(77)),
        (2, vpn, Some(77)),
        (2, vpn, Some(77)),
        (3, home, Some(77)),
        (4, cafe, Some(88)),
        (4, home, None),
    ] {
        db.record_login(hash, 10, ip, uid, &LoginDevice::default())
            .await
            .unwrap();
    }
    let links = db.linked_accounts(1, 10).await.unwrap();
    let by_name = |hash: usize| {
        links
            .iter()
            .find(|l| l.display_name == NAMES[hash - 1])
            .map(|l| (l.shared.clone(), l.shared_clients, l.logins))
    };
    // 3 shares the most, so it comes first.
    assert_eq!(links[0].display_name, "Three", "{links:?}");
    assert_eq!(by_name(3), Some((vec!["203.0.113.7".to_owned()], 1, 1)));
    assert_eq!(by_name(2), Some((Vec::new(), 1, 2)));
    // 4 only shares the address; its own install doesn't count.
    assert_eq!(by_name(4), Some((vec!["203.0.113.7".to_owned()], 0, 1)));
    // No uid links nobody.
    assert!(
        db.linked_accounts(4, 10)
            .await
            .unwrap()
            .iter()
            .all(|l| l.shared_clients == 0)
    );
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn devices_group_logins_with_their_addresses(pool: PgPool) {
    let db = Db::from_pool(pool.clone());
    for (hash, name) in [(1, "One"), (2, "Two")] {
        db.create_player(&NewPlayer {
            display_name: name,
            ..player(hash)
        })
        .await
        .unwrap();
    }
    let home: std::net::IpAddr = "203.0.113.7".parse().unwrap();
    let work: std::net::IpAddr = "198.51.100.2".parse().unwrap();
    let phone = LoginDevice {
        web: true,
        user_agent: Some("Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X)".into()),
        platform: None,
        language: Some("en-GB".into()),
        touch: true,
    };
    let desktop = LoginDevice::default();
    for (hash, ip, uid, device) in [
        (1, home, Some(5), &phone),
        (1, work, Some(5), &phone),
        (1, home, None, &desktop),
        // 2 borrows 1's phone.
        (2, home, Some(5), &phone),
    ] {
        db.record_login(hash, 10, ip, uid, device).await.unwrap();
    }
    let mut devices = db.login_devices(1, 10).await.unwrap();
    devices.sort_by_key(|d| d.logins);
    assert_eq!(devices.len(), 2, "{devices:?}");
    let (desk, mobile) = (&devices[0], &devices[1]);
    assert_eq!((desk.web, desk.uid, desk.others), (Some(false), None, 0));
    assert_eq!(desk.ips, ["203.0.113.7"]);
    assert_eq!(
        (mobile.web, mobile.touch, mobile.uid),
        (Some(true), true, Some(5))
    );
    assert_eq!((mobile.logins, mobile.addresses, mobile.others), (2, 2, 1));
    assert_eq!(mobile.language.as_deref(), Some("en-GB"));
    let mut ips = mobile.ips.clone();
    ips.sort();
    assert_eq!(ips, ["198.51.100.2", "203.0.113.7"]);
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn banned_accounts_chain_through_browsers_and_addresses(pool: PgPool) {
    let db = Db::from_pool(pool.clone());
    const NAMES: [&str; 5] = ["Mod", "Banned", "Alt", "Friend", "Far"];
    for (hash, name) in (1..).zip(NAMES) {
        db.create_player(&NewPlayer {
            display_name: name,
            staff_mod_level: if hash == 1 { 2 } else { 0 },
            ..player(hash)
        })
        .await
        .unwrap();
    }
    let ip = |n: u8| std::net::IpAddr::from([203, 0, 113, n]);
    let device = LoginDevice::default();
    // Before the ban: Banned (2) plays on browser 9 at home; Friend (4)
    // visits that home; Far (5) shares an address with Friend.
    for (hash, at, uid) in [(2, 1, Some(9)), (4, 1, None), (4, 2, None), (5, 2, None)] {
        db.record_login(hash, 10, ip(at), uid, &device)
            .await
            .unwrap();
    }
    sqlx::query("UPDATE login_logs SET created_at = now() - interval '1 day'")
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        db.moderate(1, 2, 2, ModAction::Ban { hours: 48 }, Some("Botting"), 10)
            .await
            .unwrap()
    );
    // After it: Alt (3) logs in on Banned's browser from somewhere else.
    db.record_login(3, 10, ip(3), Some(9), &device)
        .await
        .unwrap();

    let alt = db.banned_links(3).await.unwrap();
    assert_eq!(alt.len(), 1, "{alt:?}");
    assert_eq!(
        (alt[0].name.as_str(), alt[0].via.as_str()),
        ("Banned", "browser")
    );
    assert!(alt[0].after_ban && alt[0].banned_ms.is_some());
    let friend = db.banned_links(4).await.unwrap();
    assert_eq!(friend.len(), 1, "{friend:?}");
    assert_eq!(friend[0].via, "address");
    assert!(!friend[0].after_ban);
    assert!(db.banned_links(5).await.unwrap().is_empty());

    let chain = db.link_chain(2, 3, 50).await.unwrap();
    let found: Vec<(&str, u8, &str, &str)> = chain
        .iter()
        .map(|c| (c.name.as_str(), c.depth, c.via.as_str(), c.link.as_str()))
        .collect();
    assert_eq!(
        found,
        [
            ("Alt", 1, "Banned", "browser"),
            ("Friend", 1, "Banned", "address"),
            ("Far", 2, "Friend", "address"),
        ]
    );
    assert!(chain.iter().all(|c| c.banned_until_ms.is_none()));
    assert_eq!(db.link_chain(2, 1, 50).await.unwrap().len(), 2);
    assert_eq!(db.link_chain(2, 3, 1).await.unwrap().len(), 1);

    let evasion = db.ban_evasion(7, 0, 10).await.unwrap();
    assert_eq!(evasion.len(), 1, "{evasion:?}");
    assert_eq!(
        (evasion[0].name.as_str(), evasion[0].banned.as_str()),
        ("Alt", "Banned")
    );
    assert_eq!(evasion[0].logins, 1);
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn timeline_merges_every_log_and_pages(pool: PgPool) {
    let db = Db::from_pool(pool.clone());
    db.create_player(&player(42)).await.unwrap();
    db.create_player(&NewPlayer {
        staff_mod_level: 0,
        ..player(7)
    })
    .await
    .unwrap();
    db.record_login(
        7,
        10,
        "203.0.113.7".parse().unwrap(),
        None,
        &LoginDevice::default(),
    )
    .await
    .unwrap();
    let at = |x| (x, 3218, 0);
    let (x, z, y) = at(3222);
    db.record_logs(
        10,
        &LogBatch {
            chat: vec![
                ChatLog {
                    name_hash: 7,
                    to_name_hash: None,
                    x,
                    z,
                    y,
                    message: "hello".to_owned(),
                },
                ChatLog {
                    name_hash: 7,
                    to_name_hash: Some(42),
                    x,
                    z,
                    y,
                    message: "psst".to_owned(),
                },
            ],
            sessions: vec![SessionLog {
                name_hash: 7,
                x,
                z,
                y,
                event_type: 0,
                event: "Deposited coins x5 into bank".to_owned(),
            }],
            wealth: vec![WealthEvent {
                name_hash: 7,
                x,
                z,
                y,
                event_type: 5,
                item: "coins".to_owned(),
                count: 5,
                value: 5,
                counterpart: None,
            }],
        },
    )
    .await
    .unwrap();
    db.moderate(42, 2, 7, ModAction::Mute { hours: 2 }, Some("Spam"), 10)
        .await
        .unwrap();
    // One against them, one they sent.
    for (reporter, offender) in [(42, 7), (7, 42)] {
        db.record_snapshot(&NewSnapshot {
            reporter_name_hash: reporter,
            offender_name_hash: offender,
            reason: 6,
            node_id: 10,
            x,
            z,
            y,
            muted: false,
        })
        .await
        .unwrap();
    }

    let every: Vec<String> = rs_db::TIMELINE_KINDS
        .iter()
        .map(|k| k.to_string())
        .collect();
    let all = db.timeline(7, &every, None, 100).await.unwrap();
    let mut kinds: Vec<&str> = all.iter().map(|r| r.kind.as_str()).collect();
    kinds.sort();
    assert_eq!(
        kinds,
        [
            "chat",
            "chat",
            "login",
            "moderation",
            "report",
            "reported",
            "session",
            "wealth"
        ]
    );
    assert!(all.windows(2).all(|w| w[0].at_ms >= w[1].at_ms));
    let moderation = all.iter().find(|r| r.kind == "moderation").unwrap();
    assert_eq!(moderation.other.as_deref(), Some("Test"));
    assert_eq!(moderation.detail.as_deref(), Some("Spam"));
    assert_eq!(moderation.hours, Some(2));
    let login = all.iter().find(|r| r.kind == "login").unwrap();
    assert_eq!(login.text.as_deref(), Some("203.0.113.7"));

    // Paging two at a time reads the same rows in the same order, even
    // though many share a millisecond.
    let mut paged = Vec::new();
    let mut cursor: Option<TimelineCursor> = None;
    loop {
        let page = db.timeline(7, &every, cursor.as_ref(), 2).await.unwrap();
        let Some(last) = page.last() else { break };
        cursor = Some(TimelineCursor {
            at_ms: last.at_ms,
            kind: last.kind.clone(),
            id: last.id,
        });
        paged.extend(page);
    }
    let key = |r: &rs_db::TimelineRow| (r.kind.clone(), r.id);
    assert_eq!(
        paged.iter().map(key).collect::<Vec<_>>(),
        all.iter().map(key).collect::<Vec<_>>()
    );

    // Only the kinds asked for; none known reads nothing.
    let chat = db
        .timeline(7, &["chat".to_owned()], None, 100)
        .await
        .unwrap();
    assert_eq!(chat.len(), 2);
    assert_eq!(chat.iter().filter(|r| r.other_hash == Some(42)).count(), 1);
    assert!(
        db.timeline(7, &["nope".to_owned()], None, 100)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(db.timeline(99, &every, None, 100).await.unwrap().is_empty());

    // Account age comes with the credentials.
    let creds = db.find_credentials(7, 10).await.unwrap().unwrap();
    assert!((0..60_000).contains(&creds.age_ms));
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn appeals_are_filed_once_and_decided(pool: PgPool) {
    let db = Db::from_pool(pool.clone());
    db.create_player(&player(42)).await.unwrap();
    db.create_player(&NewPlayer {
        staff_mod_level: 0,
        ..player(7)
    })
    .await
    .unwrap();
    let appeal = |sanction| NewAppeal {
        name_hash: 7,
        sanction,
        statement: "It wasn't me",
        ip: Some("203.0.113.7".parse().unwrap()),
    };

    // Nothing to appeal yet.
    assert_eq!(
        db.file_appeal(&appeal("ban")).await.unwrap(),
        AppealFiled::NotSanctioned
    );
    let login = db.appeal_login(7).await.unwrap().unwrap();
    assert_eq!((login.muted_until_ms, login.banned_until_ms), (None, None));

    // A moderator's ban: the appeal knows who set it, and why.
    let ban = ModAction::Ban { hours: 48 };
    assert!(
        db.moderate(42, 2, 7, ban, Some("Macroing"), 10)
            .await
            .unwrap()
    );
    let AppealFiled::Filed { id } = db.file_appeal(&appeal("ban")).await.unwrap() else {
        panic!("expected a filed appeal");
    };
    assert_eq!(
        db.file_appeal(&appeal("ban")).await.unwrap(),
        AppealFiled::AlreadyOpen { id }
    );
    let waiting = db.waiting_appeals(10).await.unwrap();
    assert_eq!(waiting.len(), 1);
    let filed = &waiting[0];
    assert_eq!(filed.issued_by.as_deref(), Some("Test"));
    assert_eq!(filed.reason.as_deref(), Some("Macroing"));
    assert_eq!(filed.statement, "It wasn't me");
    assert!(filed.decision.is_none() && filed.case_id.is_none());

    // Taken into a case, then decided.
    assert!(db.link_appeal(id, 12).await.unwrap());
    assert!(db.waiting_appeals(10).await.unwrap().is_empty());
    assert!(
        db.decide_appeal(id, "upheld", "Stands.", "Kim")
            .await
            .unwrap()
    );
    assert!(
        !db.decide_appeal(id + 99, "upheld", "", "Kim")
            .await
            .unwrap()
    );
    let mine = db.appeals_of(7).await.unwrap();
    assert_eq!(mine[0].case_id, Some(12));
    assert_eq!(mine[0].decision.as_deref(), Some("upheld"));
    assert_eq!(mine[0].decided_by.as_deref(), Some("Kim"));

    // The same ban can't be appealed again; a new one can.
    assert_eq!(
        db.file_appeal(&appeal("ban")).await.unwrap(),
        AppealFiled::AlreadyDecided { id }
    );
    assert!(
        db.moderate(42, 2, 7, ModAction::Ban { hours: 24 }, None, 10)
            .await
            .unwrap()
    );
    assert!(matches!(
        db.file_appeal(&appeal("ban")).await.unwrap(),
        AppealFiled::Filed { .. }
    ));

    // An abuse report's mute has no moderator on record.
    db.mute_player(7, 48, 3).await.unwrap();
    let AppealFiled::Filed { id: mute } = db.file_appeal(&appeal("mute")).await.unwrap() else {
        panic!("expected a filed appeal");
    };
    let mute = db
        .appeals_of(7)
        .await
        .unwrap()
        .into_iter()
        .find(|a| a.id == mute)
        .unwrap();
    assert_eq!(mute.issued_by, None);
    assert_eq!(mute.sanction, "mute");
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn staff_above_a_level_are_left_out(pool: PgPool) {
    let db = Db::from_pool(pool.clone());
    db.create_player(&player(42)).await.unwrap();
    db.create_player(&player(43)).await.unwrap();
    // A player, and an admin.
    sqlx::query("UPDATE players SET staff_mod_level = (name_hash - 42) * 2")
        .execute(&pool)
        .await
        .unwrap();
    let chat = |name_hash, to_name_hash, message: &str| ChatLog {
        name_hash,
        to_name_hash,
        x: 3222,
        z: 3218,
        y: 0,
        message: message.to_owned(),
    };
    let logs = LogBatch {
        chat: vec![
            chat(42, None, "public"),
            chat(42, Some(43), "to the admin"),
            chat(43, None, "from the admin"),
        ],
        ..Default::default()
    };
    db.record_logs(10, &logs).await.unwrap();
    let said =
        |rows: Vec<rs_db::ChatRecord>| rows.into_iter().map(|c| c.message).collect::<Vec<_>>();
    assert_eq!(
        said(
            db.chat_history(None, None, None, None, Some(1), 10)
                .await
                .unwrap()
        ),
        ["public"]
    );
    assert_eq!(
        db.chat_history(None, None, None, None, Some(2), 10)
            .await
            .unwrap()
            .len(),
        3
    );

    let replay = |id, name_hash| NewReplay {
        id,
        name_hash,
        node_id: 10,
        revision: "289".into(),
    };
    db.start_replay(&replay(1, 42)).await.unwrap();
    db.start_replay(&replay(2, 43)).await.unwrap();
    let ids = |rows: Vec<rs_db::ReplayRecord>| rows.into_iter().map(|r| r.id).collect::<Vec<_>>();
    assert_eq!(
        ids(db.replays(None, false, None, Some(1), 10).await.unwrap()),
        [1]
    );
    assert_eq!(
        db.replays(None, false, None, None, 10).await.unwrap().len(),
        2
    );
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn chat_feeds_on_from_a_message(pool: PgPool) {
    let db = Db::from_pool(pool);
    db.create_player(&player(42)).await.unwrap();
    let chat = |message: &str| ChatLog {
        name_hash: 42,
        to_name_hash: None,
        x: 3222,
        z: 3218,
        y: 0,
        message: message.to_owned(),
    };
    let batch = |lines: &[&str]| LogBatch {
        chat: lines.iter().map(|m| chat(m)).collect(),
        ..Default::default()
    };
    db.record_logs(10, &batch(&["one", "two"])).await.unwrap();
    db.record_logs(11, &batch(&["three"])).await.unwrap();
    let newest = db
        .chat_history(None, None, None, None, None, 10)
        .await
        .unwrap();
    let first = newest.last().unwrap().id;
    let messages =
        |rows: Vec<rs_db::ChatRecord>| rows.into_iter().map(|c| c.message).collect::<Vec<_>>();
    // Newer than the first, oldest first.
    assert_eq!(
        messages(
            db.chat_history(None, None, None, Some(first), None, 10)
                .await
                .unwrap()
        ),
        ["two", "three"]
    );
    assert_eq!(
        messages(
            db.chat_history(None, Some(11), None, Some(first), None, 10)
                .await
                .unwrap()
        ),
        ["three"]
    );
    assert!(
        db.chat_history(None, None, None, Some(newest[0].id), None, 10)
            .await
            .unwrap()
            .is_empty()
    );
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn replay_chunks_page_from_a_seq(pool: PgPool) {
    let db = Db::from_pool(pool);
    db.create_player(&player(42)).await.unwrap();
    db.start_replay(&NewReplay {
        id: 1,
        name_hash: 42,
        node_id: 10,
        revision: "289".into(),
    })
    .await
    .unwrap();
    let clean = ReplayReport::default();
    // Seq 1 was empty, so never stored.
    for seq in [0, 2, 3] {
        let chunk = ReplayChunk {
            seq,
            start_ms: seq * 2000,
            end_ms: (seq + 1) * 2000,
            data: vec![seq as u8],
        };
        db.store_replay(1, Some(&chunk), (seq + 1) * 2000, &clean, false, true)
            .await
            .unwrap();
    }
    let seqs = |chunks: Vec<ReplayChunk>| chunks.into_iter().map(|c| c.seq).collect::<Vec<_>>();
    assert_eq!(seqs(db.replay_chunks_from(1, 1).await.unwrap()), [2, 3]);
    assert_eq!(seqs(db.replay_chunks_from(1, 3).await.unwrap()), [3]);
    assert!(db.replay_chunks_from(1, 4).await.unwrap().is_empty());
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn anticheat_leaders_rank_and_trend(pool: PgPool) {
    let db = Db::from_pool(pool.clone());
    for name_hash in [1, 2, 3] {
        db.create_player(&NewPlayer {
            staff_mod_level: 0,
            ..player(name_hash)
        })
        .await
        .unwrap();
    }
    let report = |score, flags: &[&str]| ReplayReport {
        score,
        flags: flags.iter().map(|f| (*f).to_owned()).collect(),
        stats: "{}".into(),
    };
    // (id, player, hours ago, score, flags)
    let sessions: [(i64, u64, i32, i16, &[&str]); 6] = [
        (1, 1, 20, 40, &["no_heartbeat"]),
        (2, 1, 2, 100, &["modified_client", "no_heartbeat"]),
        (3, 2, 5, 20, &["robotic_click_timing"]),
        (4, 2, 3, 0, &[]),
        (5, 3, 1, 0, &[]),
        // Before the window.
        (6, 3, 30, 100, &["modified_client"]),
    ];
    for (id, name_hash, hours, score, flags) in sessions {
        db.start_replay(&NewReplay {
            id,
            name_hash,
            node_id: 10,
            revision: "289".into(),
        })
        .await
        .unwrap();
        db.store_replay(id, None, 60_000, &report(score, flags), true, true)
            .await
            .unwrap();
        sqlx::query(
            "UPDATE replays SET started_at = now() - make_interval(hours => $2) WHERE id = $1",
        )
        .bind(id)
        .bind(hours)
        .execute(&pool)
        .await
        .unwrap();
    }
    let since = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64)
        - 24 * 3_600_000;
    let leaders = db.anticheat_leaders(since, 4, None, 10).await.unwrap();
    let summary: Vec<_> = leaders
        .iter()
        .map(|l| (l.sessions, l.flagged, l.max_score, l.total_score))
        .collect();
    assert_eq!(summary, [(2, 2, 100, 140), (2, 1, 20, 20)]);
    assert_eq!(leaders[0].flags, ["modified_client", "no_heartbeat"]);
    // Six-hour stretches: 20 hours ago, then 2 hours ago.
    assert_eq!(leaders[0].trend, [40, 0, 0, 100]);
    assert_eq!(leaders[1].trend, [0, 0, 0, 20]);
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn account_activity_covers_recent_accounts(pool: PgPool) {
    let db = Db::from_pool(pool.clone());
    let ip: std::net::IpAddr = "203.0.113.7".parse().unwrap();
    for name_hash in [1, 2] {
        db.create_player(&NewPlayer {
            staff_mod_level: 0,
            registration_ip: Some(ip),
            ..player(name_hash)
        })
        .await
        .unwrap();
    }
    // Staff aren't listed.
    db.create_player(&player(3)).await.unwrap();
    db.record_login(1, 10, ip, None, &LoginDevice::default())
        .await
        .unwrap();
    db.record_login(3, 10, ip, None, &LoginDevice::default())
        .await
        .unwrap();
    db.start_replay(&NewReplay {
        id: 1,
        name_hash: 1,
        node_id: 10,
        revision: "289".into(),
    })
    .await
    .unwrap();
    let report = ReplayReport {
        score: 20,
        flags: vec!["robotic_click_timing".into()],
        stats: r#"{"clickRhythm":0.05}"#.into(),
    };
    db.store_replay(1, None, 90_000, &report, true, true)
        .await
        .unwrap();
    sqlx::query("UPDATE players SET created_at = now() - interval '30 days' WHERE name_hash = 2")
        .execute(&pool)
        .await
        .unwrap();

    let since = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64)
        - 7 * 86_400_000;
    let accounts = db.account_activity(since, 10).await.unwrap();
    // Account 2 registered before the window and hasn't logged in since.
    assert_eq!(accounts.len(), 1);
    let a = &accounts[0];
    assert_eq!(a.registration_ip.as_deref(), Some("203.0.113.7"));
    assert_eq!(a.login_ips, ["203.0.113.7"]);
    assert_eq!(
        (a.sessions, a.play_ms, a.flagged, a.max_score),
        (1, 90_000, 1, 20)
    );
    assert_eq!(a.rhythm, Some(0.05));
    assert_eq!(a.hours.len(), 24);
    assert_eq!(a.hours.iter().sum::<i32>(), 1);
}

/// Moves every save point `mins` into the past.
async fn age_points(pool: &PgPool, mins: i32) {
    sqlx::query("UPDATE save_history SET saved_at = saved_at - make_interval(mins => $1)")
        .bind(mins)
        .execute(pool)
        .await
        .unwrap();
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn saves_keep_a_history_to_roll_back_to(pool: PgPool) {
    let db = Db::from_pool(pool.clone());
    db.create_player(&player(42)).await.unwrap();
    let at = |x| PlayerSave { x, ..save() };
    let kinds = |points: &[SavePoint]| points.iter().map(|p| p.kind.clone()).collect::<Vec<_>>();

    // A session's first save is kept; the next ones within the hour aren't.
    db.claim_session(42, 1, 60).await.unwrap();
    db.store_save(42, 1, &at(1)).await.unwrap();
    db.store_save(42, 1, &at(2)).await.unwrap();
    // Nor a logout soon after: the older point is from before it.
    db.store_logout_save(42, 1, &at(3)).await.unwrap();
    let points = db.save_points(42, None, None, 10).await.unwrap();
    assert_eq!(kinds(&points), ["session"]);
    let (point, kept) = db.save_point(42, points[0].id).await.unwrap().unwrap();
    assert_eq!((point.node_id, kept.x), (1, 1));
    assert_eq!(kept, at(1));

    // Later, a logout is kept, and a session save an hour on.
    age_points(&pool, 20).await;
    db.claim_session(42, 1, 60).await.unwrap();
    db.store_logout_save(42, 1, &at(4)).await.unwrap();
    db.claim_session(42, 1, 60).await.unwrap();
    db.store_save(42, 1, &at(5)).await.unwrap();
    age_points(&pool, 61).await;
    db.store_save(42, 1, &at(6)).await.unwrap();
    db.store_logout_save(42, 1, &at(7)).await.unwrap();
    let points = db.save_points(42, None, None, 10).await.unwrap();
    assert_eq!(kinds(&points), ["session", "logout", "session"]);

    // A staff change keeps the save it replaces, with who and why.
    db.claim_session(42, 1, 60).await.unwrap();
    assert_eq!(
        db.store_edit_save(42, 1, &at(8), "Jordan: rollback")
            .await
            .unwrap(),
        SaveResult::Stored
    );
    let newest = &db.save_points(42, None, None, 10).await.unwrap()[0];
    assert_eq!(
        (newest.kind.as_str(), newest.note.as_deref()),
        ("staff", Some("Jordan: rollback"))
    );
    assert_eq!(
        db.save_point(42, newest.id).await.unwrap().unwrap().1,
        at(7)
    );
    assert_eq!(db.load_save(42).await.unwrap().unwrap(), at(8));

    // Paging, and points are only another player's own.
    let older = db.save_points(42, Some(newest.id), None, 10).await.unwrap();
    assert_eq!(older.len(), 3);
    // The newest point from by a moment.
    let by = db
        .save_points(42, None, Some(older[1].saved_ms), 10)
        .await
        .unwrap();
    assert_eq!(by[0].id, older[1].id);
    db.create_player(&player(43)).await.unwrap();
    assert!(db.save_point(43, newest.id).await.unwrap().is_none());

    // Pruning keeps the newest few.
    age_points(&pool, 60 * 24 * 40).await;
    assert_eq!(db.prune_save_history(30, 2).await.unwrap(), 2);
    assert_eq!(db.save_points(42, None, None, 10).await.unwrap().len(), 2);
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn progress_is_each_point_then_the_save_now(pool: PgPool) {
    let db = Db::from_pool(pool.clone());
    db.create_player(&player(42)).await.unwrap();
    assert!(db.save_progress(42, 10).await.unwrap().is_empty());
    let with_xp = |xp| {
        let mut save = save();
        save.stats[0] = xp;
        save
    };
    db.claim_session(42, 1, 60).await.unwrap();
    db.store_save(42, 1, &with_xp(100)).await.unwrap();
    age_points(&pool, 61).await;
    db.store_save(42, 1, &with_xp(200)).await.unwrap();
    db.store_save(42, 1, &with_xp(300)).await.unwrap();
    let rows = db.save_progress(42, 10).await.unwrap();
    let seen: Vec<(&str, i32)> = rows.iter().map(|r| (r.kind.as_str(), r.stats[0])).collect();
    assert_eq!(seen, [("session", 100), ("session", 200), ("now", 300)]);
    assert!(rows.windows(2).all(|w| w[0].saved_ms <= w[1].saved_ms));
    assert_eq!(rows[2].inv_objs.len(), rows[2].inv_counts.len());
    // The newest points only, and always the save now.
    let newest = db.save_progress(42, 1).await.unwrap();
    assert_eq!(
        newest.iter().map(|r| r.stats[0]).collect::<Vec<_>>(),
        [200, 300]
    );
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn activity_counts_session_logs_by_hour(pool: PgPool) {
    let db = Db::from_pool(pool.clone());
    db.create_player(&player(42)).await.unwrap();
    db.create_player(&player(43)).await.unwrap();
    let event = |name_hash| SessionLog {
        name_hash,
        x: 3222,
        z: 3218,
        y: 0,
        event_type: 0,
        event: "Deposited coins x5 into bank".to_owned(),
    };
    let logs = LogBatch {
        sessions: vec![event(42), event(42), event(43)],
        ..Default::default()
    };
    db.record_logs(10, &logs).await.unwrap();
    // One of them from two days ago.
    sqlx::query(
        "UPDATE session_logs SET created_at = created_at - interval '2 days'
         WHERE id = (SELECT min(id) FROM session_logs)",
    )
    .execute(&pool)
    .await
    .unwrap();
    let hours = db.activity_hours(42, 30).await.unwrap();
    assert_eq!(hours.len(), 2);
    assert!(hours.iter().all(|(ms, _)| ms % 3_600_000 == 0));
    assert_eq!(hours.iter().map(|(_, n)| n).sum::<i64>(), 2);
    assert!(hours[0].0 < hours[1].0);
    assert_eq!(db.activity_hours(42, 1).await.unwrap().len(), 1);
    assert!(db.activity_hours(44, 30).await.unwrap().is_empty());
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn password_resets_are_redeemed_once(pool: PgPool) {
    let db = Db::from_pool(pool.clone());
    db.create_player(&player(42)).await.unwrap();
    assert!(db.password_reset(42).await.unwrap().is_none());
    assert!(
        !db.issue_password_reset(43, "code", 72, false)
            .await
            .unwrap()
    );

    // Without a lock the old password keeps working until it's redeemed.
    assert!(
        db.issue_password_reset(42, "code", 72, false)
            .await
            .unwrap()
    );
    let creds = db.find_credentials(42, 1).await.unwrap().unwrap();
    assert_eq!(creds.password_hash, "$argon2id$stub");
    let record = db.player_record(42).await.unwrap().unwrap();
    assert!(record.reset_until_ms.is_some());
    assert_eq!(
        db.password_reset(42).await.unwrap().unwrap().code_hash,
        "code"
    );

    // A new reset replaces it, and can lock the password now.
    assert!(db.issue_password_reset(42, "new", 72, true).await.unwrap());
    assert_eq!(
        db.find_credentials(42, 1)
            .await
            .unwrap()
            .unwrap()
            .password_hash,
        RESET_LOCKED
    );
    assert!(
        !db.redeem_password_reset(42, "code", "$argon2id$new")
            .await
            .unwrap()
    );
    assert!(
        db.redeem_password_reset(42, "new", "$argon2id$new")
            .await
            .unwrap()
    );
    assert!(
        !db.redeem_password_reset(42, "new", "$argon2id$again")
            .await
            .unwrap()
    );
    assert_eq!(
        db.find_credentials(42, 1)
            .await
            .unwrap()
            .unwrap()
            .password_hash,
        "$argon2id$new"
    );
    assert!(db.password_reset(42).await.unwrap().is_none());

    // One that ran out can't be redeemed.
    db.issue_password_reset(42, "late", 1, false).await.unwrap();
    sqlx::query("UPDATE players SET reset_code_until = now() - interval '1 minute'")
        .execute(&pool)
        .await
        .unwrap();
    assert!(db.password_reset(42).await.unwrap().is_none());
    assert!(
        !db.redeem_password_reset(42, "late", "$argon2id$late")
            .await
            .unwrap()
    );
    assert!(
        db.player_record(42)
            .await
            .unwrap()
            .unwrap()
            .reset_until_ms
            .is_none()
    );
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn only_dead_worlds_sessions_are_released(pool: PgPool) {
    let db = Db::from_pool(pool.clone());
    db.create_player(&player(42)).await.unwrap();
    assert!(db.session_owner(42).await.unwrap().is_none());
    assert_eq!(db.release_dead_session(42, 60).await.unwrap(), None);

    db.claim_session(42, 1, 60).await.unwrap();
    db.report_world_status(1, 1, false).await.unwrap();
    let owner = db.session_owner(42).await.unwrap().unwrap();
    assert_eq!(owner.node, 1);
    assert!(owner.since_ms.is_some());
    assert!(owner.heartbeat_age_ms.unwrap() < 5000);
    // Alive: kept.
    assert_eq!(db.release_dead_session(42, 60).await.unwrap(), None);

    sqlx::query("UPDATE world_status SET updated_at = now() - interval '2 minutes'")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(db.release_dead_session(42, 60).await.unwrap(), Some(1));
    assert!(db.session_owner(42).await.unwrap().is_none());

    // A world with no heartbeat at all is dead too.
    db.claim_session(42, 7, 60).await.unwrap();
    assert_eq!(
        db.session_owner(42)
            .await
            .unwrap()
            .unwrap()
            .heartbeat_age_ms,
        None
    );
    assert_eq!(db.release_dead_session(42, 60).await.unwrap(), Some(7));
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn wealth_events_are_found_by_id(pool: PgPool) {
    let db = Db::from_pool(pool);
    db.create_player(&player(42)).await.unwrap();
    db.create_player(&player(43)).await.unwrap();
    let logs = LogBatch {
        wealth: vec![WealthEvent {
            name_hash: 42,
            x: 3222,
            z: 3218,
            y: 0,
            event_type: 5,
            item: "dragon_med_helm".to_owned(),
            count: 1,
            value: 100_000,
            counterpart: None,
        }],
        ..Default::default()
    };
    db.record_logs(2, &logs).await.unwrap();
    let rows = db
        .timeline(42, &["wealth".to_owned()], None, 10)
        .await
        .unwrap();
    let id = rows[0].id;
    let found = db.wealth_event(42, id).await.unwrap().unwrap();
    assert_eq!(
        (found.item.as_str(), found.count, found.node_id),
        ("dragon_med_helm", 1, 2)
    );
    assert!(db.wealth_event(43, id).await.unwrap().is_none());
}

/// Registers a non-staff player called `name`.
async fn named(db: &Db, name_hash: u64, name: &'static str) {
    db.create_player(&NewPlayer {
        display_name: name,
        staff_mod_level: 0,
        ..player(name_hash)
    })
    .await
    .unwrap();
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn items_are_traced_back_through_players(pool: PgPool) {
    let db = Db::from_pool(pool.clone());
    named(&db, 41, "Origin").await;
    named(&db, 42, "Mid").await;
    named(&db, 43, "Mule").await;
    named(&db, 44, "Feeder").await;
    let event = |name_hash, event_type, item: &str, count, value, to| WealthEvent {
        name_hash,
        x: 3200,
        z: 3200,
        y: 0,
        event_type,
        item: item.to_owned(),
        count,
        value,
        counterpart: to,
    };
    let logs = LogBatch {
        wealth: vec![
            event(41, 6, "rune_axe", 1, 2000, None),
            event(41, transfer::DROP, "rune_axe", 1, 2000, None),
            event(42, transfer::TAKE, "rune_axe", 1, 2000, None),
            event(42, transfer::TRADE, "rune_axe", 1, 2000, Some(43)),
            event(44, transfer::TRADE, "coins", 1000, 1000, Some(43)),
            event(42, transfer::STAKE, "coins", 500, 500, Some(43)),
            event(43, transfer::TRADE, "lobster", 1, 10, Some(42)),
        ],
        ..Default::default()
    };
    db.record_logs(10, &logs).await.unwrap();
    // A minute apart, in the order they happened; then a trade of two items
    // at once, which is one handover.
    sqlx::query(
        "UPDATE wealth_events SET created_at = now()
             - make_interval(mins => (SELECT max(id) FROM wealth_events)::int - id::int + 1)",
    )
    .execute(&pool)
    .await
    .unwrap();

    let axe = ["rune_axe".to_owned(), "cert_rune_axe".to_owned()];
    let both = LogBatch {
        wealth: vec![
            event(44, transfer::TRADE, "coins", 100, 100, Some(43)),
            event(44, transfer::TRADE, "bread", 1, 4, Some(43)),
        ],
        ..Default::default()
    };
    db.record_logs(10, &both).await.unwrap();

    let sources = db.item_sources(43, &axe, None, 10).await.unwrap();
    assert_eq!(sources.len(), 1);
    assert_eq!(
        (sources[0].event_type, sources[0].from_name.as_deref()),
        (transfer::TRADE, Some("Mid"))
    );

    // Mule had it from Mid, who picked up what Origin dropped, who bought it.
    let trace = db.item_trace(43, &axe, 4, 10, 100, None).await.unwrap();
    let steps: Vec<_> = trace
        .iter()
        .map(|s| {
            (
                s.holder.as_str(),
                s.depth,
                s.parent,
                s.source.event_type,
                s.source.from_name.as_deref(),
            )
        })
        .collect();
    assert_eq!(
        steps,
        [
            ("Mule", 0, None, transfer::TRADE, Some("Mid")),
            ("Mid", 1, Some(0), transfer::TAKE, Some("Origin")),
            ("Origin", 2, Some(1), 6, None),
        ]
    );
    // Not back through staff above the one looking.
    sqlx::query("UPDATE players SET staff_mod_level = 2 WHERE name_hash = 42")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        db.item_trace(43, &axe, 4, 10, 100, Some(1))
            .await
            .unwrap()
            .len(),
        1
    );
    // Only so far back.
    assert_eq!(
        db.item_trace(43, &axe, 1, 10, 100, None)
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(
        db.item_trace(99, &axe, 4, 10, 100, None)
            .await
            .unwrap()
            .is_empty()
    );

    // Who hands Mule what, and what passes between their partners.
    let graph = db.trade_graph(43, 0, 10).await.unwrap();
    let names: Vec<_> = graph.accounts.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(names, ["Mule", "Mid", "Feeder"]);
    let edges: Vec<_> = graph
        .edges
        .iter()
        .map(|e| (e.from.as_str(), e.to.as_str(), e.value, e.handovers))
        .collect();
    assert_eq!(
        edges,
        [
            ("Mid", "Mule", 2500, 2),
            ("Feeder", "Mule", 1104, 2),
            ("Mule", "Mid", 10, 1)
        ]
    );
    assert_eq!(
        graph.edges[0].items,
        [("rune_axe".to_owned(), 1), ("coins".to_owned(), 500)]
    );

    // Fed by two, giving little back.
    let receivers = db.top_receivers(0, 2, 10).await.unwrap();
    assert_eq!(receivers.len(), 1);
    assert_eq!(
        (
            receivers[0].name.as_str(),
            receivers[0].givers,
            receivers[0].received,
            receivers[0].given
        ),
        ("Mule", 2, 3604, 10)
    );
    assert_eq!(receivers[0].handovers, 4);

    // Mule's timeline has what they were handed, and from whom.
    let rows = db
        .timeline(43, &["wealth".to_owned()], None, 10)
        .await
        .unwrap();
    let received: Vec<_> = rows
        .iter()
        .filter(|r| r.detail.as_deref() == Some("received"))
        .map(|r| (r.text.as_deref(), r.other.as_deref()))
        .collect();
    assert_eq!(
        received,
        [
            (Some("bread"), Some("Feeder")),
            (Some("coins"), Some("Feeder")),
            (Some("coins"), Some("Mid")),
            (Some("coins"), Some("Feeder")),
            (Some("rune_axe"), Some("Mid"))
        ]
    );
    let gave = rows.iter().find(|r| r.detail.is_none()).unwrap();
    assert_eq!(
        (gave.text.as_deref(), gave.other.as_deref()),
        (Some("lobster"), Some("Mid"))
    );
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn holders_and_totals_come_from_saves(pool: PgPool) {
    let db = Db::from_pool(pool);
    named(&db, 42, "Rich").await;
    named(&db, 43, "Saver").await;
    db.claim_session(42, 1, 60).await.unwrap();
    db.store_logout_save(42, 1, &save()).await.unwrap();
    let saver = PlayerSave {
        inv_types: vec![93, 95],
        inv_slots: vec![0, 0],
        inv_objs: vec![995, 995],
        inv_counts: vec![50, 100],
        ..save()
    };
    db.claim_session(43, 1, 60).await.unwrap();
    db.store_logout_save(43, 1, &saver).await.unwrap();

    let holders = db.item_holders(&[995], 1, 10).await.unwrap();
    let seen: Vec<_> = holders
        .iter()
        .map(|h| {
            (
                h.name.as_str(),
                h.total,
                h.inv_types.clone(),
                h.inv_counts.clone(),
            )
        })
        .collect();
    assert_eq!(
        seen,
        [
            (
                "Rich",
                i64::from(i32::MAX),
                vec![93],
                vec![i64::from(i32::MAX)]
            ),
            ("Saver", 150, vec![95, 93], vec![100, 50]),
        ]
    );
    assert_eq!(db.item_holders(&[995], 151, 10).await.unwrap().len(), 1);
    assert_eq!(
        db.item_totals(&[995, 1351, 4151]).await.unwrap(),
        [
            rs_db::ItemTotal {
                obj: 995,
                total: i64::from(i32::MAX) + 150,
                holders: 2
            },
            rs_db::ItemTotal {
                obj: 1351,
                total: 1,
                holders: 1
            },
        ]
    );
}

#[sqlx::test]
#[ignore = "needs DATABASE_URL"]
async fn staff_keep_players_off_the_hiscores(pool: PgPool) {
    let db = Db::from_pool(pool.clone());
    ranked(&db, 1, "Maxed", 0, &[(0, 130_344_310)]).await;
    ranked(&db, 2, "Cheater", 0, &[(0, 372_240), (10, 372_240)]).await;
    // Player moderators are ranked; Jagex moderators and maintainers aren't.
    ranked(&db, 3, "Jmod", 2, &[(0, 200_000_000)]).await;
    ranked(&db, 4, "Dev", 3, &[(0, 200_000_000)]).await;
    ranked(&db, 5, "Pmod", 1, &[(0, 100)]).await;
    assert_eq!(
        names(&db.hiscores_page(1, 0, 10).await.unwrap()),
        ["Maxed", "Cheater", "Pmod"]
    );

    // Their attack entry removed: still on the others.
    assert!(db.change_hiscores(2, None, &[1], &[]).await.unwrap());
    assert_eq!(
        names(&db.hiscores_page(1, 0, 10).await.unwrap()),
        ["Maxed", "Pmod"]
    );
    assert_eq!(
        names(&db.hiscores_page(11, 0, 10).await.unwrap()),
        ["Cheater"]
    );
    let standing = db.hiscore_standing(2).await.unwrap().unwrap();
    assert_eq!(
        (standing.hidden, standing.removed.clone()),
        (false, vec![1])
    );
    let rank = |category| {
        standing
            .rows
            .iter()
            .find(|r| r.category == category)
            .unwrap()
            .rank
    };
    assert_eq!((rank(1), rank(11)), (None, Some(1)));

    // Hidden from every table, then let back on.
    db.change_hiscores(2, Some(true), &[], &[]).await.unwrap();
    assert!(db.hiscores_page(11, 0, 10).await.unwrap().is_empty());
    assert!(db.player_hiscores(2).await.unwrap().is_none());
    let kept = db.hiscores_kept_off(10).await.unwrap();
    assert_eq!(
        kept.iter()
            .map(|k| (k.name.as_str(), k.hidden, k.removed.clone()))
            .collect::<Vec<_>>(),
        [("Cheater", true, vec![1])]
    );
    db.change_hiscores(2, Some(false), &[], &[1]).await.unwrap();
    assert_eq!(
        names(&db.hiscores_page(1, 0, 10).await.unwrap()),
        ["Maxed", "Cheater", "Pmod"]
    );
    assert!(db.hiscores_kept_off(10).await.unwrap().is_empty());
    assert!(!db.change_hiscores(99, Some(true), &[], &[]).await.unwrap());
    assert!(db.hiscore_standing(99).await.unwrap().is_none());
}
