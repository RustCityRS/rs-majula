use crate::save::set_chat_filters;
use crate::{Db, Error, LobbyVisit};
use sqlx::{FromRow, PgConnection, query, query_as, query_scalar};
use std::net::IpAddr;

/// Duration for which a departing client can finish its world login.
const HANDOFF_SECS: i32 = 15;

/// Lobby and world nodes have independent u8 IDs. Reserve negative session owners for lobbies.
pub fn lobby_node(id: u8) -> i16 {
    -1 - i16::from(id)
}

pub fn lobby_id(owner: i16) -> Option<u8> {
    u8::try_from(-1 - i32::from(owner)).ok()
}

#[derive(Clone, Debug)]
pub struct WorldEndpoint {
    pub id: u16,
    pub revision: i32,
    pub address: String,
    pub members: bool,
    pub location: String,
    pub country: u16,
    pub activity: Option<String>,
}

#[derive(Clone, Debug, FromRow)]
pub struct LobbyWorld {
    pub id: i16,
    pub address: String,
    pub members: bool,
    pub players: i32,
    pub location: String,
    pub country: i16,
    pub activity: Option<String>,
}

/// An account as the lobby shows it: [`Db::lobby_account`]. Dates are days
/// since 1970 and times milliseconds since 1970, `None` for none. An
/// account is underage while its date of birth is under 13 years ago.
#[derive(Clone, Debug, FromRow)]
pub struct AccountDetails {
    pub display_name: String,
    pub gender: Option<i16>,
    pub underage: bool,
    pub chat_consent: bool,
    pub advert_consent: bool,
    pub date_of_birth_days: Option<i32>,
    pub dob_requested: bool,
    pub subscribed: bool,
    pub members_until_ms: Option<i64>,
    pub loyalty_points: i32,
    pub email_status: i16,
    pub account_guardian: bool,
    pub card_expires_days: Option<i32>,
    pub grace_expires_days: Option<i32>,
}

/// Clears the world endpoint of the `world_status` rows `$where` picks.
macro_rules! clear_endpoint {
    ($where:literal) => {
        concat!(
            "UPDATE world_status SET revision = NULL, world_id = NULL, address = NULL, members = NULL,
                     location = NULL, country = NULL, activity = NULL WHERE ",
            $where
        )
    };
}

/// Ends the session of every player online at lobby `node`.
async fn release_lobby_players(conn: &mut PgConnection, node: i16) -> Result<(), Error> {
    query("UPDATE players SET online_node = NULL, online_since = NULL WHERE online_node = $1")
        .bind(node)
        .execute(conn)
        .await?;
    Ok(())
}

impl Db {
    /// The details the lobby shows for an account, with its display name and
    /// the gender in its save. `None` if no such player.
    pub async fn lobby_account(&self, name_hash: u64) -> Result<Option<AccountDetails>, Error> {
        query_as(
            concat!("SELECT p.display_name, s.gender,
                    coalesce(d.date_of_birth > current_date - interval '13 years', false) AS underage,
                    d.chat_consent, d.advert_consent,
                    d.date_of_birth - DATE '1970-01-01' AS date_of_birth_days,
                    d.dob_requested, d.subscribed,
                    ", ms!("d.members_until"), " AS members_until_ms,
                    d.loyalty_points, d.email_status, d.account_guardian,
                    d.card_expires - DATE '1970-01-01' AS card_expires_days,
                    d.grace_expires - DATE '1970-01-01' AS grace_expires_days
             FROM players p
             JOIN account_details d ON d.player_id = p.id
             LEFT JOIN player_saves s ON s.player_id = p.id
             WHERE p.name_hash = $1"),
        )
        .bind(name_hash as i64)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn clear_world_endpoint(&self, node: u8) -> Result<(), Error> {
        query(clear_endpoint!("node_id = $1 AND revision IS NOT NULL"))
            .bind(i16::from(node))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn advertise_world(
        &self,
        node: u8,
        world: &WorldEndpoint,
        max_age: i32,
    ) -> Result<(), Error> {
        let mut tx = self.pool.begin().await?;
        // Stock clients index worlds by number within one revision. Two hosts advertising
        // the same number would direct different clients to different owners of that slot.
        query("SELECT pg_advisory_xact_lock($1, $2)")
            .bind(world.revision)
            .bind(i32::from(world.id))
            .execute(&mut *tx)
            .await?;
        let conflict: bool = query_scalar(concat!(
            "SELECT EXISTS(SELECT 1 FROM world_status
                WHERE revision = $1 AND world_id = $2 AND node_id <> $3
                AND ",
            live!("updated_at", "$4"),
            ")"
        ))
        .bind(world.revision)
        .bind(world.id as i16)
        .bind(i16::from(node))
        .bind(max_age)
        .fetch_one(&mut *tx)
        .await?;
        if conflict {
            return Err(Error::Protocol(format!(
                "World {} is already advertised for revision {}",
                world.id, world.revision
            )));
        }
        query(clear_endpoint!(
            "revision = $1 AND world_id = $2 AND node_id <> $3"
        ))
        .bind(world.revision)
        .bind(world.id as i16)
        .bind(i16::from(node))
        .execute(&mut *tx)
        .await?;
        query(
            "UPDATE world_status SET world_id = $2, revision = $3, address = $4,
                     members = $5, location = $6, country = $7, activity = $8 WHERE node_id = $1",
        )
        .bind(i16::from(node))
        .bind(world.id as i16)
        .bind(world.revision)
        .bind(&world.address)
        .bind(world.members)
        .bind(&world.location)
        .bind(world.country as i16)
        .bind(&world.activity)
        .execute(&mut *tx)
        .await?;
        tx.commit().await
    }

    pub async fn lobby_worlds(
        &self,
        revision: i32,
        max_age: i32,
    ) -> Result<Vec<LobbyWorld>, Error> {
        query_as(
            concat!("SELECT world_id AS id, address, members, players, location, country, activity FROM world_status
                        WHERE node_id >= 0 AND revision = $1 AND NOT maintenance
                          AND ", live!("updated_at", "$2"), "
                        ORDER BY world_id"),
        )
        .bind(revision)
        .bind(max_age)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn start_lobby(&self, node: i16, instance: i64, max_age: i32) -> Result<bool, Error> {
        let mut tx = self.pool.begin().await?;
        let acquired = query(concat!(
            "INSERT INTO world_status (node_id, players, lobby_instance)
                VALUES ($1, 0, $2) ON CONFLICT (node_id) DO UPDATE SET lobby_instance = $2,
                updated_at = now(), players = 0
                WHERE NOT (",
            live!("world_status.updated_at", "$3"),
            ")"
        ))
        .bind(node)
        .bind(instance)
        .bind(max_age)
        .execute(&mut *tx)
        .await?
        .rows_affected()
            == 1;
        if acquired {
            release_lobby_players(&mut tx, node).await?;
        }
        tx.commit().await?;
        Ok(acquired)
    }

    pub async fn renew_lobby(
        &self,
        node: i16,
        instance: i64,
        players: i32,
        max_age: i32,
    ) -> Result<bool, Error> {
        Ok(query(concat!(
            "UPDATE world_status SET updated_at = now(), players = $3
                        WHERE node_id = $1 AND lobby_instance = $2
                          AND ",
            live!("updated_at", "$4")
        ))
        .bind(node)
        .bind(instance)
        .bind(players)
        .bind(max_age)
        .execute(&self.pool)
        .await?
        .rows_affected()
            == 1)
    }

    pub async fn leave_lobby(
        &self,
        name: u64,
        node: i16,
        instance: i64,
        visit: Option<(LobbyVisit, IpAddr, Option<u8>)>,
        chat_filters: Option<[i16; 3]>,
    ) -> Result<(), Error> {
        let mut tx = self.pool.begin().await?;
        let current = query(
            "SELECT 1 FROM world_status WHERE node_id = $1 AND lobby_instance = $2 FOR SHARE",
        )
        .bind(node)
        .bind(instance)
        .fetch_optional(&mut *tx)
        .await?;
        if current.is_some() {
            let left: Option<i64> = query_scalar(
                "UPDATE players SET online_node = NULL, online_since = NULL,
                lobby_session = $3, lobby_ip = $4::inet, lobby_revision = $5, lobby_target = $6,
                lobby_until = CASE WHEN $3 IS NOT NULL THEN now() + make_interval(secs => $7) END
                WHERE name_hash = $1 AND online_node = $2 RETURNING id",
            )
            .bind(name as i64)
            .bind(node)
            .bind(visit.map(|v| v.0.client_session))
            .bind(visit.map(|v| v.1.to_canonical().to_string()))
            .bind(visit.map(|v| v.0.revision))
            .bind(visit.and_then(|v| v.2).map(i16::from))
            .bind(HANDOFF_SECS)
            .fetch_optional(&mut *tx)
            .await?;
            if let (Some(id), Some(filters)) = (left, chat_filters) {
                set_chat_filters(&mut tx, id, filters).await?;
            }
        }
        tx.commit().await
    }

    pub async fn stop_lobby(&self, node: i16, instance: i64) -> Result<(), Error> {
        let mut tx = self.pool.begin().await?;
        let ended = query("DELETE FROM world_status WHERE node_id = $1 AND lobby_instance = $2")
            .bind(node)
            .bind(instance)
            .execute(&mut *tx)
            .await?;
        if ended.rows_affected() == 1 {
            release_lobby_players(&mut tx, node).await?;
        }
        tx.commit().await
    }
}
