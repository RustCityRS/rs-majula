use crate::MAX_PLAYERS;
use crate::active_player::ActivePlayer;
use crate::core::{Engine, next_free_id};
use rs_datastruct::HashTable;
use rs_pack::types::BlockWalk;
use rustc_hash::FxHashMap;

/// The players in the world, held in `Engine::player_list`: a slot per pid,
/// the order the phases process them in, a user37 index to find a pid by
/// name, and how many isolated staff are online.
pub struct PlayerList {
    pub players: Vec<Option<ActivePlayer>>,
    pub processing: HashTable<u16>,
    node_map: Vec<usize>,
    by_user37: FxHashMap<u64, u16>,
    cursor: u16,
    pid_scratch: Vec<u16>,
    pub(super) isolated: u16,
}

impl PlayerList {
    /// An empty list with every pid slot free, set so the first
    /// [`next_pid`](Self::next_pid) hands out pid 1.
    pub fn new() -> Self {
        let mut players = Vec::with_capacity(MAX_PLAYERS);
        players.resize_with(MAX_PLAYERS, || None);
        Self {
            players,
            processing: HashTable::new(8),
            node_map: vec![0; MAX_PLAYERS],
            by_user37: FxHashMap::default(),
            cursor: (MAX_PLAYERS - 2) as u16,
            pid_scratch: Vec::with_capacity(MAX_PLAYERS),
            isolated: 0,
        }
    }

    /// Takes the reusable pid buffer, filled with the current
    /// processing-order pids. A stable owned snapshot is required because the
    /// phase loops may remove entries (emergency removal) mid-iteration.
    /// Return it with [`put_pids`](Self::put_pids) to reuse the allocation.
    pub fn take_pids(&mut self) -> Vec<u16> {
        let mut v = std::mem::take(&mut self.pid_scratch);
        v.clear();
        v.extend(self.processing.iter().copied());
        v
    }

    /// Returns the buffer taken by [`take_pids`](Self::take_pids) for reuse.
    pub fn put_pids(&mut self, v: Vec<u16>) {
        self.pid_scratch = v;
    }

    /// The next free pid, searched round robin from just after the last pid
    /// added; `None` when the list is full. Pids run from 1 to
    /// `MAX_PLAYERS - 2`.
    pub fn next_pid(&self) -> Option<u16> {
        next_free_id(self.cursor, (MAX_PLAYERS - 1) as u16, 1, |i| {
            self.players[i as usize].is_none()
        })
    }

    /// Puts `active` in slot `pid`, links it into the processing order under
    /// `key`, indexes it by user37 and counts it if it is isolated staff. The
    /// next [`next_pid`](Self::next_pid) search starts after `pid`.
    pub fn add(&mut self, pid: u16, active: ActivePlayer, key: i64) {
        self.cursor = pid;
        let node_idx = self.processing.put(key, pid);
        self.node_map[pid as usize] = node_idx;
        self.isolated += active.player.staff_mod_level.is_isolated() as u16;
        self.by_user37.insert(active.player.uid.username37(), pid);
        self.players[pid as usize] = Some(active);
    }

    /// Empties slot `pid` and returns its player, or `None` if it was empty:
    /// unlinks it from the processing order, drops its user37 entry if that
    /// still names `pid`, and uncounts it if it is isolated staff.
    pub fn remove(&mut self, pid: u16) -> Option<ActivePlayer> {
        let active = self.players[pid as usize].take()?;
        self.processing.unlink(self.node_map[pid as usize]);
        self.isolated -= active.player.staff_mod_level.is_isolated() as u16;
        let user37 = active.player.uid.username37();
        if self.by_user37.get(&user37) == Some(&pid) {
            self.by_user37.remove(&user37);
        }
        Some(active)
    }

    /// The player in slot `pid`, if any.
    pub fn get(&self, pid: u16) -> Option<&ActivePlayer> {
        self.players.get(pid as usize)?.as_ref()
    }

    /// The player in slot `pid`, mutably, if any.
    pub fn get_mut(&mut self, pid: u16) -> Option<&mut ActivePlayer> {
        self.players.get_mut(pid as usize)?.as_mut()
    }

    /// Player `pid` in its slot, and every other slot.
    pub fn split(&mut self, pid: u16) -> Option<(&mut ActivePlayer, OtherPlayers<'_>)> {
        let (before, rest) = self.players.split_at_mut(pid as usize);
        let (slot, after) = rest.split_first_mut()?;
        Some((slot.as_mut()?, OtherPlayers { before, after }))
    }

    /// The pid of the player in the list named `user37`, if any.
    pub fn find_by_user37(&self, user37: u64) -> Option<u16> {
        let pid = *self.by_user37.get(&user37)?;
        self.players[pid as usize].is_some().then_some(pid)
    }

    /// Every pid in processing order, as a new `Vec`;
    /// [`take_pids`](Self::take_pids) reuses one buffer instead.
    pub fn pids(&self) -> Vec<u16> {
        self.processing.iter().copied().collect()
    }

    /// How many players are in the list.
    pub fn count(&self) -> usize {
        self.processing.len()
    }
}

/// The player slots but one, so the output phase can encode a player in its
/// slot while reading the others. The missing slot reads as empty.
#[derive(Clone, Copy)]
pub struct OtherPlayers<'a> {
    before: &'a [Option<ActivePlayer>],
    after: &'a [Option<ActivePlayer>],
}

impl<'a> OtherPlayers<'a> {
    /// Every slot.
    pub fn all(players: &'a [Option<ActivePlayer>]) -> Self {
        Self {
            before: players,
            after: &[],
        }
    }

    /// The player in slot `pid`, if any; the left-out slot reads as `None`.
    pub fn get(&self, pid: u16) -> Option<&'a ActivePlayer> {
        let pid = pid as usize;
        let split = self.before.len();
        match self.before.get(pid) {
            Some(slot) => slot.as_ref(),
            None => self.after.get(pid.wrapping_sub(split + 1))?.as_ref(),
        }
    }
}

impl Engine {
    /// Puts a player into the world in slot `pid`, which must be free (as
    /// [`PlayerList::next_pid`] gives): into the player list and into the zone
    /// at its coord.
    ///
    /// # Arguments
    ///
    /// * `key` -- The processing-order key: players are processed grouped by
    ///   its low three bits, then in the order they were added. Logins pass
    ///   the client's IPv4 address (an IPv6 address's last four bytes), bots
    ///   their pid.
    ///
    /// # Side Effects
    ///
    /// - Sets `pathing.presence` to its coord.
    /// - Fills slot `pid` through [`PlayerList::add`]: the processing order,
    ///   the user37 index and the isolated staff count.
    /// - Registers the pid in the zone at its coord.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `accept_login`, `spawn_bots` (`::bots`, `::localbots`).
    /// **Calls:** [`PlayerList::add`], `Zone::add_player`.
    pub fn add_player(&mut self, pid: u16, mut active: ActivePlayer, key: i64) {
        active.player.pathing.presence = active.player.pathing.coord;
        self.player_list.add(pid, active, key);
        if let Some(active) = self.get_player(pid) {
            self.zones
                .zone_mut(
                    active.player.pathing.coord.x(),
                    active.player.pathing.coord.y(),
                    active.player.pathing.coord.z(),
                )
                .add_player(pid);
        }
    }

    /// Takes player `pid` out of the world and the player list. Saves nothing
    /// and tells ether nothing; callers that need that run `save_logout` or
    /// `release_logout` alongside it.
    ///
    /// # Returns
    ///
    /// The removed player, or `None` if slot `pid` was empty.
    ///
    /// # Side Effects
    ///
    /// - Drops the pid's player renderer state and clears its player
    ///   snapshot, so observers still to come this tick encode its removal.
    /// - Removes the pid from the zone at its `presence`, and clears the NPC
    ///   and player collision flags its `block_walk` puts there.
    /// - Decrements `observers` on every NPC in its build area.
    /// - Frees slot `pid` through [`PlayerList::remove`].
    ///
    /// # Call Stack
    ///
    /// **Called by:** `logouts`, `emergency_remove_player`,
    /// `drop_unowned_player`, `process_shutdown` (bots) and `harness_reset`.
    /// **Calls:** `PlayerRenderer::remove_permanent`,
    /// `PlayerSnapshots::clear`, [`PlayerList::remove`].
    pub fn remove_player(&mut self, pid: u16) -> Option<ActivePlayer> {
        self.player_renderer.remove_permanent(pid);
        // Mark absent in the hot-field snapshot so any observer still processed
        // later this tick encodes a remove for this pid (mirrors the slot going
        // `None`). Harmless outside the info/output phases; refilled next tick.
        self.player_snapshots.clear(pid);
        if let Some(active) = self.player_list.get(pid) {
            let coord = active.player.pathing.presence;
            let nids: Vec<u16> = active.player.build_area.npcs.iter().to_vec();
            self.zones
                .zone_mut(coord.x(), coord.y(), coord.z())
                .remove_player(pid);

            for nid in nids {
                if let Some(npc) = self
                    .npc_list
                    .npcs
                    .get_mut(nid as usize)
                    .and_then(|n| n.as_mut())
                {
                    npc.npc.observers = npc.npc.observers.saturating_sub(1);
                }
            }

            let block_walk = active.player.block_walk;
            let size = active.player.pathing.size;
            match block_walk {
                BlockWalk::Npc => {
                    rsmod::change_npc(coord.x(), coord.z(), coord.y(), size, false);
                }
                BlockWalk::All => {
                    rsmod::change_npc(coord.x(), coord.z(), coord.y(), size, false);
                    rsmod::change_player(coord.x(), coord.z(), coord.y(), size, false);
                }
                _ => {}
            }
        }
        self.player_list.remove(pid)
    }

    /// The player in slot `pid` of the player list, if any.
    pub fn get_player(&self, pid: u16) -> Option<&ActivePlayer> {
        self.player_list.get(pid)
    }

    /// The player in slot `pid` of the player list, mutably, if any.
    pub fn get_player_mut(&mut self, pid: u16) -> Option<&mut ActivePlayer> {
        self.player_list.get_mut(pid)
    }

    /// Forcibly removes a player from the engine, saving their profile and
    /// notifying the ether service.
    ///
    /// Used as a last resort when a player's session is in an unrecoverable
    /// state (e.g. a panic during their tick processing). Saves the player
    /// through [`Engine::save_logout`], which logs them out of ether once the
    /// save lands, then delegates to [`Engine::remove_player`].
    ///
    /// # Arguments
    ///
    /// * `pid` -- The player id to force-remove.
    ///
    /// # Side Effects
    ///
    /// - Saves the player profile and logs them out of ether once it lands
    ///   (see [`Engine::save_logout`]).
    /// - Calls `remove_player` for full cleanup.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `fence`, `process_shutdown`, `cycle` after a fatal phase
    /// panic, and panic recovery in the input, player, info and output phases.
    /// **Calls:** `save_logout`, `remove_player`.
    pub fn emergency_remove_player(&mut self, pid: u16) {
        self.save_logout(pid);
        self.remove_player(pid);
    }

    /// Sends a game message to every currently online player.
    ///
    /// If the text contains newlines, each line is sent as a separate message.
    ///
    /// # Arguments
    ///
    /// * `text` -- The message text to broadcast.
    ///
    /// # Side Effects
    ///
    /// Calls `message_game_wrapped` on every active player.
    pub fn broadcast(&mut self, text: &str) {
        for active in self.player_list.players.iter_mut().flatten() {
            if text.contains('\n') {
                for line in text.split('\n') {
                    active.message_game_wrapped(line);
                }
            } else {
                active.message_game_wrapped(text);
            }
        }
    }

    /// Finds the pid of an online player by their Base37-encoded username.
    ///
    /// A map lookup ([`PlayerList::find_by_user37`]): object reveals and
    /// every ether message (PMs, friend updates) call this.
    ///
    /// # Arguments
    ///
    /// * `user37` -- Base37-encoded username to search for.
    ///
    /// # Returns
    ///
    /// `Some(pid)` if a matching player is online, `None` otherwise.
    pub fn find_pid_by_user37(&self, user37: u64) -> Option<u16> {
        self.player_list.find_by_user37(user37)
    }

    /// Scales a duration inversely by the current online player count.
    ///
    /// Used to make respawn timers shorter when more players are online,
    /// preventing resource starvation on busy worlds. The formula is
    /// `(4000 - min(player_count, 2000)) * rate / 4000`, so at 0 players the
    /// full rate is returned, and at 2000 players the rate is halved.
    ///
    /// # Arguments
    ///
    /// * `rate` -- The base duration in game ticks.
    ///
    /// # Returns
    ///
    /// The scaled duration, always less than or equal to `rate`.
    pub fn scale_by_player_count(&self, rate: u64) -> u64 {
        let player_count = self.player_list.count().min(2000) as u64;
        (4000 - player_count) * rate / 4000
    }
}
