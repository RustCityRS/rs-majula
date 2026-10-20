use crate::active_player::{ActivePlayer, EnginePlayer};
use crate::clients::client_db::{DbRequest, LoadResult, SaveOutcome};
use crate::engine::Engine;
use crate::player_save::{PlayerProfile, PlayerProfileInv};
use rs_grid::CoordGrid;
use rs_inv::{Inventory, StackMode};
use rs_pack::cache::CacheStore;
use rs_pack::cache::inv::{InvScope, InvType};
use rs_pack::cache::provider::CacheType;
use rs_pack::types::PlayerStat;
use rs_stat::{get_exp_by_level, get_level_by_exp};
use rs_vm::engine::ScriptPlayer;
use tokio::sync::oneshot;
use tracing::error;

/// A player's stats, items and position.
pub struct Sheet {
    /// Logged in here; otherwise this is their save.
    pub online: bool,
    pub x: u16,
    pub z: u16,
    pub level: u8,
    pub stats: Vec<SheetStat>,
    pub invs: Vec<SheetInv>,
}

pub struct SheetStat {
    pub id: u8,
    pub name: String,
    pub level: u8,
    pub base: u8,
    pub xp: i32,
}

pub struct SheetInv {
    pub name: String,
    pub size: u16,
    pub items: Vec<SheetItem>,
}

pub struct SheetItem {
    pub slot: u16,
    pub id: u16,
    pub name: String,
    pub count: u32,
}

pub enum Change {
    SetStat { stat: u8, level: u8 },
    Give { inv: String, obj: u16, count: u32 },
    Take { inv: String, obj: u16, count: u32 },
    Move { x: u16, z: u16, level: u8 },
    Restore { items: Vec<Restored> },
    SendHome,
}

pub struct Restored {
    pub inv: String,
    pub obj: u16,
    pub count: u32,
}

/// Where players respawn, in Lumbridge.
const HOME: (u16, u16) = (3221, 3218);
/// Where new players start, on Tutorial Island.
const TUTORIAL_START: (u16, u16) = (3094, 3106);

/// Where to send a player stuck at `x`, `z`: Lumbridge, or the start of
/// Tutorial Island for one still on it, who'd skip it otherwise. The island
/// and its caves as the content's `in_tutorial_island` has them.
pub fn home_for(x: u16, z: u16) -> (u16, u16, u8) {
    let on_island = ((3053..=3156).contains(&x) && (3056..=3136).contains(&z))
        || ((3072..=3118).contains(&x) && (9492..=9535).contains(&z));
    let (x, z) = if on_island { TUTORIAL_START } else { HOME };
    (x, z, 0)
}

pub struct ObjMatch {
    pub id: u16,
    pub name: String,
    pub debugname: String,
    pub stackable: bool,
}

/// The inventories always listed, even empty, for items to be given into.
const LISTED_INVS: [&str; 3] = ["inv", "worn", "bank"];

/// The stats players have (the two unused slots left out).
fn stats() -> impl Iterator<Item = (u8, String)> {
    (0..21u8).filter_map(|id| {
        let stat = PlayerStat::try_from(id).ok()?;
        let name = format!("{stat:?}");
        (!name.starts_with("Stat")).then_some((id, name))
    })
}

fn obj_name(cache: &CacheStore, id: u16) -> String {
    cache.objs.get_by_id(id).map_or_else(
        || format!("obj_{id}"),
        |obj| {
            obj.name
                .as_deref()
                .or(obj.debugname())
                .map_or_else(|| format!("obj_{id}"), str::to_owned)
        },
    )
}

/// A save's XP, total level and wealth (see [`Engine::progress`]).
pub struct Progress {
    pub xp: i64,
    pub total_level: u32,
    pub wealth: i64,
}

/// A saved inventory, by its config name.
fn saved_inv<'a>(cache: &'a CacheStore, name: &str) -> Result<&'a InvType, String> {
    cache
        .invs
        .get_by_debugname(name)
        .filter(|inv| inv.scope == InvScope::Perm)
        .ok_or_else(|| format!("There's no saved inventory called {name}."))
}

fn empty_inv(inv: &InvType) -> Inventory {
    let mode = if inv.stackall {
        StackMode::Always
    } else {
        StackMode::Normal
    };
    Inventory::with_stack_mode(inv.size as usize, mode)
}

fn sheet_inv(cache: &CacheStore, inv: &InvType, items: Vec<(u16, u16, u32)>) -> SheetInv {
    SheetInv {
        name: inv.debugname().unwrap_or("?").to_owned(),
        size: inv.size,
        items: items
            .into_iter()
            .map(|(slot, id, count)| SheetItem {
                slot,
                id,
                name: obj_name(cache, id),
                count,
            })
            .collect(),
    }
}

/// The saved inventories to list: those with items, and [`LISTED_INVS`].
fn listed_invs(
    cache: &CacheStore,
    has_items: impl Fn(u16) -> bool,
) -> impl Iterator<Item = &InvType> {
    (0..cache.invs.count() as u16)
        .filter_map(|id| cache.invs.get_by_id(id))
        .filter(move |inv| {
            inv.scope == InvScope::Perm
                && (has_items(inv.id) || inv.debugname().is_some_and(|n| LISTED_INVS.contains(&n)))
        })
}

fn check_coord(x: u16, z: u16, level: u8) -> Result<(), String> {
    if level > 3 || x >= 16384 || z >= 16384 {
        return Err("That isn't a tile on the map.".to_owned());
    }
    Ok(())
}

fn check_level(level: u8) -> Result<(), String> {
    if !(1..=99).contains(&level) {
        return Err("Levels are 1 to 99.".to_owned());
    }
    Ok(())
}

/// Gives into `inventory`, saying how it went.
fn give(
    cache: &CacheStore,
    inventory: &mut Inventory,
    obj: u16,
    count: u32,
) -> Result<String, String> {
    let Some(obj_type) = cache.objs.get_by_id(obj) else {
        return Err(format!("There's no item {obj}."));
    };
    if count == 0 {
        return Err("Give at least 1.".to_owned());
    }
    let left = inventory.add(obj, count, obj_type.stackable);
    let name = obj_name(cache, obj);
    match left {
        0 => Ok(format!("Gave {count} x {name}.")),
        _ if left == count => Err(format!("There's no room for {name}.")),
        _ => Ok(format!(
            "Gave {} x {name}; {left} didn't fit.",
            count - left
        )),
    }
}

/// Gives all of `count` into `inventory`, or fails; says what was given.
fn give_all(
    cache: &CacheStore,
    inventory: &mut Inventory,
    inv: &str,
    obj: u16,
    count: u32,
) -> Result<String, String> {
    let Some(obj_type) = cache.objs.get_by_id(obj) else {
        return Err(format!("There's no item {obj}."));
    };
    if count == 0 {
        return Err("Restore at least 1 of each item.".to_owned());
    }
    let name = obj_name(cache, obj);
    if inventory.add(obj, count, obj_type.stackable) > 0 {
        return Err(format!(
            "There's no room for {count} x {name} in their {inv}; nothing was restored."
        ));
    }
    Ok(format!("{count} x {name}"))
}

fn restored(given: &[String]) -> String {
    format!("Restored {}.", given.join(", "))
}

/// Takes from `inventory`, saying how it went.
fn take(
    cache: &CacheStore,
    inventory: &mut Inventory,
    obj: u16,
    count: u32,
) -> Result<String, String> {
    let name = obj_name(cache, obj);
    match inventory.delete(obj, count) {
        0 => Err(format!("They have no {name} there.")),
        taken => Ok(format!("Took {taken} x {name}.")),
    }
}

impl Engine {
    /// The sheet of a player logged in here.
    pub fn player_sheet(&self, user37: u64) -> Option<Sheet> {
        let active = self
            .find_pid_by_user37(user37)
            .and_then(|pid| self.player_list.players[pid as usize].as_ref())?;
        let player = &active.player;
        let coord = player.pathing.coord;
        let cache = self.cache;
        Some(Sheet {
            online: true,
            x: coord.x(),
            z: coord.z(),
            level: coord.y(),
            stats: stats()
                .map(|(id, name)| SheetStat {
                    id,
                    name,
                    level: player.stats.levels[id as usize] as u8,
                    base: player.stats.base_levels[id as usize] as u8,
                    xp: player.stats.xp[id as usize] / 10,
                })
                .collect(),
            invs: listed_invs(cache, |id| {
                player
                    .invs
                    .get(&id)
                    .is_some_and(|inv| inv.slots.iter().any(Option::is_some))
            })
            .map(|inv| {
                let items = player.invs.get(&inv.id).map_or_else(Vec::new, |items| {
                    items
                        .slots
                        .iter()
                        .enumerate()
                        .filter_map(|(slot, item)| {
                            item.as_ref().map(|i| (slot as u16, i.obj, i.num))
                        })
                        .collect()
                });
                sheet_inv(cache, inv, items)
            })
            .collect(),
        })
    }

    /// The sheet of a player from their save.
    pub fn profile_sheet(&self, profile: &PlayerProfile) -> Sheet {
        let cache = self.cache;
        Sheet {
            online: false,
            x: profile.x,
            z: profile.z,
            level: profile.y,
            stats: stats()
                .map(|(id, name)| SheetStat {
                    id,
                    name,
                    level: profile.levels[id as usize],
                    base: get_level_by_exp(profile.stats[id as usize]),
                    xp: profile.stats[id as usize] / 10,
                })
                .collect(),
            invs: listed_invs(cache, |id| profile.invs.iter().any(|i| i.inv_type == id))
                .map(|inv| {
                    let items = profile
                        .invs
                        .iter()
                        .find(|i| i.inv_type == inv.id)
                        .map_or_else(Vec::new, |i| i.items.clone());
                    sheet_inv(cache, inv, items)
                })
                .collect(),
        }
    }

    /// A save's progress: its stats' XP (`stats` is 10x, as saved) and
    /// total level, and what its items (`objs` with `counts` alongside) are
    /// worth at their value in the cache, unknown ones counting for nothing.
    pub fn progress(&self, stats: &[i32], objs: &[i32], counts: &[i32]) -> Progress {
        let trained = || self::stats().filter_map(|(id, _)| stats.get(id as usize).copied());
        let wealth = objs
            .iter()
            .zip(counts)
            .filter_map(|(&obj, &count)| {
                let obj = self.cache.objs.get_by_id(u16::try_from(obj).ok()?)?;
                Some(i64::from(obj.cost.max(0)) * i64::from(count.max(0)))
            })
            .sum();
        Progress {
            xp: trained().map(|xp| i64::from(xp.max(0) / 10)).sum(),
            total_level: trained()
                .map(|xp| u32::from(get_level_by_exp(xp.max(0))))
                .sum(),
            wealth,
        }
    }

    /// Makes `change` to a player logged in here; `None` if they aren't.
    pub fn edit_player(&mut self, user37: u64, change: &Change) -> Option<Result<String, String>> {
        let cache = self.cache;
        let pid = self.find_pid_by_user37(user37)?;
        let active = self.get_player_mut(pid)?;
        Some(edit_online(cache, active, change))
    }

    /// Makes `change` to a save.
    pub fn edit_profile(
        &self,
        profile: &mut PlayerProfile,
        change: &Change,
    ) -> Result<String, String> {
        let cache = self.cache;
        match *change {
            Change::SetStat { stat, level } => {
                check_level(level)?;
                let name = stat_name(stat)?;
                profile.stats[stat as usize] = get_exp_by_level(level);
                profile.levels[stat as usize] = level;
                Ok(format!("Set {name} to {level}."))
            }
            Change::Give {
                ref inv,
                obj,
                count,
            } => {
                let inv_type = giftable_inv(cache, inv)?;
                with_profile_inv(profile, inv_type, |inventory| {
                    give(cache, inventory, obj, count)
                })
            }
            Change::Take {
                ref inv,
                obj,
                count,
            } => {
                let inv_type = saved_inv(cache, inv)?;
                with_profile_inv(profile, inv_type, |inventory| {
                    take(cache, inventory, obj, count)
                })
            }
            Change::Move { x, z, level } => {
                check_coord(x, z, level)?;
                profile.x = x;
                profile.z = z;
                profile.y = level;
                Ok(format!("Moved them to {x}, {z} (level {level})."))
            }
            Change::Restore { ref items } => {
                // A save that fails half way is never stored.
                let mut given = Vec::with_capacity(items.len());
                for item in items {
                    let inv_type = giftable_inv(cache, &item.inv)?;
                    given.push(with_profile_inv(profile, inv_type, |inventory| {
                        give_all(cache, inventory, &item.inv, item.obj, item.count)
                    })?);
                }
                Ok(restored(&given))
            }
            Change::SendHome => {
                let (x, z, level) = home_for(profile.x, profile.z);
                profile.x = x;
                profile.z = z;
                profile.y = level;
                Ok(format!("They'll log in at {x}, {z}."))
            }
        }
    }

    /// Starts an edit of a logged-out player: holds their name here and asks
    /// the database to claim their session and load their save. Refused while
    /// they're logged in here, logging in or out, or already being edited.
    /// Every start must be ended with [`Engine::end_offline_edit`].
    pub fn begin_offline_edit(
        &mut self,
        user37: u64,
    ) -> Result<oneshot::Receiver<LoadResult>, String> {
        if self.find_pid_by_user37(user37).is_some() {
            return Err("They just logged in here; try again.".to_owned());
        }
        if self.logout_saves.contains_key(&user37)
            || self.pending_logins.iter().any(|p| p.user37 == user37)
        {
            return Err("They're logging in or out; try again in a moment.".to_owned());
        }
        if self.offline_edits.contains(&user37) {
            return Err("Someone else is changing them right now; try again.".to_owned());
        }
        let Some(tx) = &self.db_tx else {
            return Err("This world has no database.".to_owned());
        };
        let (reply, answer) = oneshot::channel();
        tx.send(DbRequest::EditLoad { user37, reply })
            .map_err(|_| "This world's database is down.".to_owned())?;
        self.offline_edits.insert(user37);
        Ok(answer)
    }

    /// Stores an edited save, which ends the edit's claim. The save it
    /// replaces is kept in the save history, with `note` saying who and why.
    pub fn store_offline_edit(
        &self,
        user37: u64,
        profile: PlayerProfile,
        note: String,
    ) -> Option<oneshot::Receiver<SaveOutcome>> {
        let (reply, answer) = oneshot::channel();
        self.db_tx
            .as_ref()?
            .send(DbRequest::EditStore {
                user37,
                profile: Box::new(profile),
                note,
                reply,
            })
            .ok()?;
        Some(answer)
    }

    /// Ends an edit begun with [`Engine::begin_offline_edit`]. Unless its save
    /// was stored, the claim (if the load took one) is released: the database
    /// handles requests in order, so the release lands after the claim.
    pub fn end_offline_edit(&mut self, user37: u64, stored: bool) {
        if !self.offline_edits.remove(&user37) {
            return;
        }
        if !stored && let Some(tx) = &self.db_tx {
            let _ = tx.send(DbRequest::Release { user37 });
        }
    }

    /// Items whose name or config name contains `query`, at most `limit`.
    pub fn find_objs(&self, query: &str, limit: usize) -> Vec<ObjMatch> {
        let query = query.trim().to_lowercase();
        if query.is_empty() {
            return Vec::new();
        }
        let objs = &self.cache.objs;
        let exact = query.parse::<u16>().ok();
        let mut found: Vec<_> = (0..objs.count() as u16)
            .filter_map(|id| objs.get_by_id(id))
            .filter(|obj| {
                Some(obj.id) == exact
                    || obj.debugname().is_some_and(|n| n.contains(&query))
                    || obj
                        .name
                        .as_deref()
                        .is_some_and(|n| n.to_lowercase().contains(&query))
            })
            .collect();
        // The item itself before others that only mention it (`coins`
        // before `fake_coins`).
        found.sort_by_key(|obj| {
            (
                Some(obj.id) != exact,
                obj.debugname() != Some(query.as_str()),
                obj.name.as_deref().map(str::to_lowercase).as_deref() != Some(query.as_str()),
                obj.id,
            )
        });
        found
            .into_iter()
            .take(limit)
            .map(|obj| ObjMatch {
                id: obj.id,
                name: obj_name(self.cache, obj.id),
                debugname: obj.debugname().unwrap_or_default().to_owned(),
                stackable: obj.stackable,
            })
            .collect()
    }
}

fn stat_name(stat: u8) -> Result<String, String> {
    stats()
        .find(|&(id, _)| id == stat)
        .map(|(_, name)| name)
        .ok_or_else(|| format!("There's no stat {stat}."))
}

/// A saved inventory items can be given into: worn items belong in their
/// equipment slot, which a plain add doesn't pick.
fn giftable_inv<'a>(cache: &'a CacheStore, name: &str) -> Result<&'a InvType, String> {
    if name == "worn" {
        return Err("Items can't be given straight into worn equipment.".to_owned());
    }
    saved_inv(cache, name)
}

/// Runs `f` on one of a save's inventories, as the game would load it, and
/// writes it back.
fn with_profile_inv(
    profile: &mut PlayerProfile,
    inv: &InvType,
    f: impl FnOnce(&mut Inventory) -> Result<String, String>,
) -> Result<String, String> {
    let mut inventory = empty_inv(inv);
    let saved = profile.invs.iter().position(|i| i.inv_type == inv.id);
    if let Some(index) = saved {
        for &(slot, obj, count) in &profile.invs[index].items {
            if (slot as usize) < inventory.capacity {
                inventory.set(slot, obj, count);
            }
        }
    }
    let message = f(&mut inventory)?;
    let items: Vec<(u16, u16, u32)> = inventory
        .slots
        .iter()
        .enumerate()
        .filter_map(|(slot, item)| item.as_ref().map(|i| (slot as u16, i.obj, i.num)))
        .collect();
    match saved {
        Some(index) if items.is_empty() => {
            profile.invs.remove(index);
        }
        Some(index) => profile.invs[index].items = items,
        None if items.is_empty() => {}
        None => profile.invs.push(PlayerProfileInv {
            inv_type: inv.id,
            items,
        }),
    }
    Ok(message)
}

fn edit_online(
    cache: &CacheStore,
    active: &mut ActivePlayer,
    change: &Change,
) -> Result<String, String> {
    match *change {
        Change::SetStat { stat, level } => {
            check_level(level)?;
            let name = stat_name(stat)?;
            let stat = stat as usize;
            active.player.stats.base_levels[stat] = level as u16;
            active.player.stats.levels[stat] = level as u16;
            active.player.stats.xp[stat] = get_exp_by_level(level);
            active.recalc_combat_and_appearance();
            Ok(format!("Set {name} to {level}."))
        }
        Change::Give {
            ref inv,
            obj,
            count,
        } => {
            let inv_type = giftable_inv(cache, inv)?;
            let inventory = active
                .player
                .invs
                .entry(inv_type.id)
                .or_insert_with(|| empty_inv(inv_type));
            give(cache, inventory, obj, count)
        }
        Change::Take {
            ref inv,
            obj,
            count,
        } => {
            let inv_type = saved_inv(cache, inv)?;
            let Some(inventory) = active.player.invs.get_mut(&inv_type.id) else {
                return Err(format!("They have no {} there.", obj_name(cache, obj)));
            };
            let message = take(cache, inventory, obj, count)?;
            if inv == "worn" {
                active.recalc_combat_and_appearance();
            }
            Ok(message)
        }
        Change::Move { x, z, level } => {
            check_coord(x, z, level)?;
            teleport(active, x, z, level)?;
            Ok(format!("Moved them to {x}, {z} (level {level})."))
        }
        Change::Restore { ref items } => {
            // Into copies first, so it's all or nothing.
            let mut staged: Vec<(u16, Inventory)> = Vec::new();
            let mut given = Vec::with_capacity(items.len());
            for item in items {
                let inv_type = giftable_inv(cache, &item.inv)?;
                let index = match staged.iter().position(|(id, _)| *id == inv_type.id) {
                    Some(index) => index,
                    None => {
                        let current = active.player.invs.get(&inv_type.id).cloned();
                        staged.push((inv_type.id, current.unwrap_or_else(|| empty_inv(inv_type))));
                        staged.len() - 1
                    }
                };
                given.push(give_all(
                    cache,
                    &mut staged[index].1,
                    &item.inv,
                    item.obj,
                    item.count,
                )?);
            }
            for (id, inventory) in staged {
                active.player.invs.insert(id, inventory);
            }
            Ok(restored(&given))
        }
        Change::SendHome => {
            let coord = active.player.pathing.coord;
            let (x, z, level) = home_for(coord.x(), coord.z());
            teleport(active, x, z, level)?;
            Ok(format!("Sent them to {x}, {z}."))
        }
    }
}

fn teleport(active: &mut ActivePlayer, x: u16, z: u16, level: u8) -> Result<(), String> {
    if let Err(e) = active.close_modal(true) {
        error!("error closing modal for a console move: {e}");
    }
    if !active.can_access() {
        return Err(
            "They're busy (in a cutscene or dialogue); try again, or kick them and do it while they're logged out."
                .to_owned(),
        );
    }
    active.player.clear_interaction();
    active.unset_map_flag();
    active.tele_jump(CoordGrid::new(x, level, z));
    Ok(())
}
