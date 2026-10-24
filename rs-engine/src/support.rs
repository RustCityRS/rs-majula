use crate::clients::client_db::{DbRequest, SyncOutcome};
use crate::engine::Engine;
use crate::player_save::has_save_file;
use rs_util::base37::to_raw_username;
use tokio::sync::oneshot;

/// What this world holds a player's session for.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct SessionHold {
    pub online: bool,
    pub logging_in: bool,
    pub logout_save: Option<bool>,
    pub editing: bool,
    pub local_save: bool,
}

/// What [`Engine::unstick`] did.
pub enum Unstick {
    Released,
    Resent,
    Syncing(oneshot::Receiver<SyncOutcome>),
}

impl Engine {
    /// How this world holds `user37`'s session.
    pub fn session_hold(&self, user37: u64) -> SessionHold {
        SessionHold {
            online: self.find_pid_by_user37(user37).is_some(),
            logging_in: self.pending_logins.iter().any(|p| p.user37 == user37),
            logout_save: self.logout_saves.get(&user37).map(|save| save.failed),
            editing: self.offline_edits.contains(&user37),
            local_save: has_save_file(&to_raw_username(user37)),
        }
    }

    /// Frees `user37`'s session on this world when they can't log in:
    /// whatever still holds it is finished, or the claim let go. Refused
    /// while it's in use (they're online, logging in, or being changed).
    pub fn unstick(&mut self, user37: u64) -> Result<Unstick, String> {
        let hold = self.session_hold(user37);
        if hold.online {
            return Err(
                "They're online here. If they're stuck in the game, send them home or kick them."
                    .to_owned(),
            );
        }
        if hold.logging_in {
            return Err("They're logging in here right now.".to_owned());
        }
        if hold.editing {
            return Err("Staff are changing their save right now; try again.".to_owned());
        }
        match hold.logout_save {
            Some(true) => {
                self.send_logout_save(user37);
                return Ok(Unstick::Resent);
            }
            Some(false) => {
                return Err(
                    "Their logout save is on its way to the database; try again in a moment."
                        .to_owned(),
                );
            }
            None => {}
        }
        let Some(tx) = &self.db_tx else {
            return Err("This world has no database.".to_owned());
        };
        if hold.local_save {
            let (reply, answer) = oneshot::channel();
            tx.send(DbRequest::SyncLocal { user37, reply })
                .map_err(|_| "This world's database is down.".to_owned())?;
            return Ok(Unstick::Syncing(answer));
        }
        self.release_claim(user37);
        Ok(Unstick::Released)
    }
}
