use crate::active_player::{ActivePlayer, EnginePlayer};
use crate::clients::client_game::create_io;
use crate::engine::{cache, engine, engine_mut};
use crate::handlers::ClientGameHandler;
use core::str::Split;
use num_enum::TryFromPrimitive;
use rs_crypto::isaac::IsaacPair;
use rs_entity::StaffModLevel;
use rs_grid::CoordGrid;
use rs_info::Visibility;
use rs_pack::cache::category::CategoryType;
use rs_pack::cache::r#enum::EnumType;
use rs_pack::cache::idk::IdkType;
use rs_pack::cache::r#if::IfType;
use rs_pack::cache::inv::InvType;
use rs_pack::cache::loc::LocType;
use rs_pack::cache::npc::NpcType;
use rs_pack::cache::obj::ObjType;
use rs_pack::cache::provider::CacheType;
use rs_pack::cache::seq::SeqType;
use rs_pack::cache::spotanim::SpotAnimType;
use rs_pack::cache::r#struct::StructType;
use rs_pack::cache::varp::VarPlayerType;
use rs_pack::cache::{ScriptVarType, VarValue};
use rs_pack::types::{DummyItem, LocAngle, LocShape, NpcStat, PlayerStat};
use rs_protocol::network::game::client::client_cheat::ClientCheat;
use rs_stat::get_exp_by_level;
use rs_vm::ScriptError;
use rs_vm::engine::{ScriptEngine, ScriptPlayer};
use rs_vm::state::ScriptArgument;
use rs_vm::subject::ScriptSubject;
use std::panic;
use tracing::error;

/// Handles the `ClientCheat` client protocol message.
///
/// Processes developer/admin cheat commands entered via the client console.
/// The cheat string is validated for length (max 80 characters), parsed into
/// a command name and arguments, then dispatched based on the player's staff
/// moderation level. Access is cumulative: each level can run its own commands
/// plus those of every level below it
/// (`Developer` > `JagexModerator` > `PlayerModerator` > `Normal`).
///
/// # Arguments
///
/// * `active` - The active player whose client sent this message.
///
/// # Returns
///
/// * `Ok(())` on success or if the player lacks permissions.
/// * `Err(ScriptError)` if the cheat string is too long, empty, or a command
///   execution error occurs.
///
/// # Side Effects
///
/// * Dispatches to the handler for the player's staff level, which cascades to
///   lower levels for any command it does not handle.
///
/// # Call Stack
///
/// **Called by:** `ActivePlayer::decode_and_handle` (via `ClientGameHandler` dispatch)
/// **Calls:** [`cheat_normal`], [`cheat_player_moderator`], [`cheat_jagex_moderator`],
///   or [`cheat_developer`] depending on the player's staff level
impl ClientGameHandler for ClientCheat {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        if self.cheat.len() > 80 {
            return Err(ScriptError::Client(format!(
                "Client cheat command was too long: {}",
                self.cheat.len()
            )));
        }

        let input_lower = self.cheat.to_lowercase();
        let mut args = input_lower.split(' ');
        let cmd = args.next().unwrap_or_default();
        if cmd.is_empty() {
            return Err(ScriptError::Client(format!(
                "Client cheat command was not found for input: {}",
                self.cheat
            )));
        }

        match active.player.staff_mod_level {
            StaffModLevel::Normal => cheat_normal(cmd, args, active),
            StaffModLevel::PlayerModerator => cheat_player_moderator(cmd, args, active),
            StaffModLevel::JagexModerator => cheat_jagex_moderator(cmd, args, active),
            StaffModLevel::Developer => cheat_developer(cmd, args, active),
        }
    }
}

/// Dispatches developer-level cheat commands, falling through to
/// [`cheat_jagex_moderator`] (and the levels below it) for any command this
/// level does not handle.
///
/// # Arguments
///
/// * `cmd` - The lowercased command name (first token of the cheat input).
/// * `args` - An iterator over the remaining space-separated argument tokens.
/// * `active` - The active player executing the cheat command.
///
/// # Returns
///
/// * `Ok(())` on success or for unrecognized commands.
/// * `Err(ScriptError)` if argument parsing fails or a script lookup/execution error occurs.
///
/// # Side Effects
///
/// * May modify engine state (reload, clock rate, spawning bots).
/// * May modify player inventory, variables, or map objects.
///
/// # Call Stack
///
/// **Called by:** `ClientCheat::handle`
/// **Calls:** `engine_mut().run_script_by_name`, `parse_*` helpers, [`cheat_spawn_bots`]
fn cheat_developer(
    cmd: &str,
    mut args: Split<'_, char>,
    active: &mut ActivePlayer,
) -> Result<(), ScriptError> {
    match cmd {
        _ if cmd.starts_with("~") => cheat_debugproc(&cmd, &mut args, active),
        "reload" => cheat_reload(),
        "give" => cheat_give(&mut args, active),
        "givemany" => cheat_give_many(&mut args, active),
        "setvar" => cheat_setvar(&mut args, active),
        "getvar" => cheat_getvar(&mut args, active),
        "setstat" => cheat_set_stat(&mut args, active),
        "advancestat" => cheat_advance_stat(&mut args, active),
        "minme" => cheat_minme(active),
        "givecrap" => cheat_give_crap(active),
        "speed" => cheat_engine_speed(&mut args, active),
        "reboot" => cheat_reboot(),
        "slowreboot" => cheat_slow_reboot(&mut args),
        "bots" => cheat_spawn_bots(active),
        "locadd" => cheat_loc_add(&mut args, active),
        "npcadd" => cheat_npc_add(&mut args, active),
        _ => cheat_jagex_moderator(cmd, args, active),
    }
}

/// Dispatches jagex-moderator cheat commands, falling through to
/// [`cheat_player_moderator`] (and below) for any command it does not handle.
///
/// Supported commands:
/// - `setvis <level>` - Sets the player's visibility (0 = default, 1 = soft, 2 = hard).
/// - `getcoord` - Prints the player's current coordinate.
/// - `tele <level,mx,mz[,lx,lz]>` - Teleports the player to a coordinate.
///
/// Reached directly by jagex moderators, and by developers cascading down from
/// [`cheat_developer`].
fn cheat_jagex_moderator(
    cmd: &str,
    mut args: Split<'_, char>,
    active: &mut ActivePlayer,
) -> Result<(), ScriptError> {
    match cmd {
        "setvis" => cheat_setvis(&mut args, active),
        "getcoord" => cheat_getcoord(active),
        "tele" => cheat_tele(&mut args, active),
        _ => cheat_player_moderator(cmd, args, active),
    }
}

/// Dispatches player-moderator cheat commands, falling through to
/// [`cheat_normal`] for any command it does not handle.
///
/// Reached directly by player moderators, and by higher staff cascading down.
fn cheat_player_moderator(
    cmd: &str,
    args: Split<'_, char>,
    active: &mut ActivePlayer,
) -> Result<(), ScriptError> {
    match cmd {
        _ => cheat_normal(cmd, args, active),
    }
}

/// Dispatches cheat commands available to every player; unrecognized commands
/// are ignored.
///
/// This is the base of the staff-level fall-through chain that all higher
/// levels cascade down to.
fn cheat_normal(
    cmd: &str,
    _args: Split<'_, char>,
    _active: &mut ActivePlayer,
) -> Result<(), ScriptError> {
    match cmd {
        _ => Ok(()),
    }
}

/// Runs the `[debugproc,<name>]` script, parsing the remaining arguments into
/// the script's declared parameter types (int, obj, loc, coord, stat, ...).
///
/// Usage: `::~<name> [args...]` (e.g. `::~getcoord` or `::~setxp attack 50`).
fn cheat_debugproc(
    cmd: &&str,
    args: &mut Split<char>,
    active: &mut ActivePlayer,
) -> Result<(), ScriptError> {
    let name = format!("[debugproc,{}]", &cmd[1..]);
    let Some(script) = engine().scripts.get_by_name(&name).cloned() else {
        return Err(ScriptError::ScriptNotFoundName(name));
    };

    let mut params = Vec::with_capacity(script.info.params.len());

    for index in 0..script.info.params.len() {
        let var_type =
            ScriptVarType::try_from_primitive(script.info.params[index]).map_err(|_| {
                ScriptError::Client(format!(
                    "Invalid script var type: {}",
                    script.info.params[index]
                ))
            })?;

        match var_type {
            ScriptVarType::Int => {
                let value = args.next().unwrap_or("0");
                params.push(ScriptArgument::Int(value.parse::<i32>().unwrap_or(0)));
            }
            ScriptVarType::String => {
                params.push(ScriptArgument::String(args.next().unwrap_or("").into()))
            }
            ScriptVarType::Enum => parse_enum(args.next(), |v| {
                params.push(ScriptArgument::Int(v.id as i32))
            })?,
            ScriptVarType::Obj | ScriptVarType::NamedObj => parse_obj(args.next(), |v| {
                params.push(ScriptArgument::Int(v.id as i32))
            })?,
            ScriptVarType::Loc => parse_loc(args.next(), |v| {
                params.push(ScriptArgument::Int(v.id as i32))
            })?,
            ScriptVarType::Struct => parse_struct(args.next(), |v| {
                params.push(ScriptArgument::Int(v.id as i32))
            })?,
            ScriptVarType::Boolean => {
                parse_bool(args.next(), |v| params.push(ScriptArgument::Int(v as i32)))?
            }
            ScriptVarType::Coord => parse_coord(args.next(), |v| {
                params.push(ScriptArgument::Int(v.packed() as i32))
            })?,
            ScriptVarType::Category => parse_category(args.next(), |v| {
                params.push(ScriptArgument::Int(v.id as i32))
            })?,
            ScriptVarType::Spotanim => parse_spotanim(args.next(), |v| {
                params.push(ScriptArgument::Int(v.id as i32))
            })?,
            ScriptVarType::Npc => parse_npc(args.next(), |v| {
                params.push(ScriptArgument::Int(v.id as i32))
            })?,
            ScriptVarType::Inv => parse_inv(args.next(), |v| {
                params.push(ScriptArgument::Int(v.id as i32))
            })?,
            ScriptVarType::Seq => parse_seq(args.next(), |v| {
                params.push(ScriptArgument::Int(v.id as i32))
            })?,
            ScriptVarType::Stat => {
                parse_stat(args.next(), |v| params.push(ScriptArgument::Int(v as i32)))?
            }
            ScriptVarType::Interface => parse_interface(args.next(), |v| {
                params.push(ScriptArgument::Int(v.id as i32))
            })?,
            ScriptVarType::NpcStat => {
                parse_npcstat(args.next(), |v| params.push(ScriptArgument::Int(v as i32)))?
            }
            ScriptVarType::Idkit => parse_idk(args.next(), |v| {
                params.push(ScriptArgument::Int(v.id as i32))
            })?,
            ScriptVarType::AutoInt => {}
            ScriptVarType::Component => {}
            ScriptVarType::Synth => {}
            ScriptVarType::Varp => {}
            ScriptVarType::PlayerUid => {}
            ScriptVarType::NpcUid => {}
            ScriptVarType::DbRow => {}
        }
    }

    engine_mut().run_script_by_name(
        &name,
        Some(ScriptSubject::Player(active.player.uid)),
        None,
        None,
        None,
        Some(params.into_iter().collect::<Vec<ScriptArgument>>()),
    )
}

/// Signals the engine to hot-reload its scripts and cache data.
///
/// Usage: `::reload` (no arguments).
fn cheat_reload() -> Result<(), ScriptError> {
    let _ = engine_mut().reload_tx.send(());
    Ok(())
}

/// Adds an object to the player's backpack (`inv`), defaulting to a count of 1.
///
/// Usage: `::give <obj> [count]` (e.g. `::give coins 1000`).
fn cheat_give(args: &mut Split<char>, active: &mut ActivePlayer) -> Result<(), ScriptError> {
    let obj_name = args.next().unwrap_or_default();
    let obj = cache()
        .objs
        .get_by_debugname(obj_name)
        .ok_or(ScriptError::ObjNotFoundName(obj_name.into()))?;
    let inv = cache()
        .invs
        .get_by_debugname("inv")
        .ok_or(ScriptError::InvNotFoundName("inv".into()))?;
    if let Some(inventory) = active.player.invs.get_mut(&inv.id) {
        let count = args.next().unwrap_or("1").parse::<i32>().unwrap_or(1);
        inventory.add(obj.id, count as u32, obj.stackable);
    }
    Ok(())
}

/// Adds 1000 of an object to the player's backpack (`inv`).
///
/// Usage: `::givemany <obj>` (e.g. `::givemany coins`).
fn cheat_give_many(args: &mut Split<char>, active: &mut ActivePlayer) -> Result<(), ScriptError> {
    let obj_name = args.next();
    let obj = cache()
        .objs
        .get_by_debugname(obj_name.unwrap_or_default())
        .ok_or(ScriptError::ObjNotFoundName(
            obj_name.unwrap_or_default().into(),
        ))?;
    let inv = cache()
        .invs
        .get_by_debugname("inv")
        .ok_or(ScriptError::InvNotFoundName("inv".into()))?;
    if let Some(inventory) = active.player.invs.get_mut(&inv.id) {
        inventory.add(obj.id, 1000, obj.stackable);
    }
    Ok(())
}

/// Sets a stat's base and current level (clamped 1-99) and syncs its experience
/// to that level, recalculating combat level and appearance if they change.
///
/// Usage: `::setstat <stat> <level>` (e.g. `::setstat attack 60`).
fn cheat_set_stat(args: &mut Split<char>, active: &mut ActivePlayer) -> Result<(), ScriptError> {
    parse_stat(args.next(), |stat| {
        let level = args
            .next()
            .unwrap_or("1")
            .parse::<i32>()
            .unwrap_or(1)
            .clamp(1, 99) as u8;
        let stat = stat as usize;
        active.player.stats.base_levels[stat] = level;
        active.player.stats.levels[stat] = level;
        active.player.stats.xp[stat] = get_exp_by_level(level);
        active.recalc_combat_and_appearance();
    })
}

/// Advances a stat to a target level via the authentic experience path: resets it
/// to level 1, then awards the experience for `<level>` through `add_xp` so the
/// normal level-up effects fire (stat sync, AdvanceStat script, combat recalc).
///
/// Usage: `::advancestat <stat> <level>` (e.g. `::advancestat woodcutting 50`).
fn cheat_advance_stat(
    args: &mut Split<char>,
    active: &mut ActivePlayer,
) -> Result<(), ScriptError> {
    parse_stat(args.next(), |stat| {
        let level = args
            .next()
            .unwrap_or("1")
            .parse::<i32>()
            .unwrap_or(1)
            .clamp(1, 99) as u8;
        let stat = stat as usize;
        active.player.stats.xp[stat] = 0;
        active.player.stats.base_levels[stat] = 1;
        active.player.stats.levels[stat] = 1;
        active.add_xp(stat, get_exp_by_level(level));
    })
}

/// Sets every stat to its minimum: Hitpoints to level 10, all others to level 1.
///
/// Usage: `::minme` (no arguments).
fn cheat_minme(active: &mut ActivePlayer) -> Result<(), ScriptError> {
    let n = active.player.stats.levels.len();
    for stat in 0..n {
        let level: u8 = if stat == PlayerStat::Hitpoints as usize {
            10
        } else {
            1
        };
        active.player.stats.base_levels[stat] = level;
        active.player.stats.levels[stat] = level;
        active.player.stats.xp[stat] = get_exp_by_level(level);
    }
    active.recalc_combat_and_appearance();
    Ok(())
}

/// Spawns a temporary loc (centrepiece-straight, facing west) at the player's tile
/// that despawns after 500 ticks, then reports its id.
///
/// Usage: `::locadd <loc>` (e.g. `::locadd dead_tree`).
fn cheat_loc_add(args: &mut Split<char>, active: &mut ActivePlayer) -> Result<(), ScriptError> {
    let name = args.next();
    parse_loc(name, |loc_type| {
        let coord = active.player.pathing.coord;
        engine_mut().add_or_change_loc(
            coord,
            loc_type.id,
            LocShape::CentrepieceStraight,
            LocAngle::West,
            500,
            true,
        );
        active.message_game(&format!(
            "Loc Added: {} (ID: {})",
            name.unwrap_or_default(),
            loc_type.id
        ));
    })
}

/// Spawns a temporary NPC at the player's tile with a despawn lifecycle.
///
/// Usage: `::npcadd <npc>` (e.g. `::npcadd man`).
fn cheat_npc_add(args: &mut Split<char>, active: &mut ActivePlayer) -> Result<(), ScriptError> {
    parse_npc(args.next(), |npc_type| {
        let coord = active.player.pathing.coord.packed();
        engine_mut().add_npc_spawned(coord, npc_type.id, 500);
    })
}

/// Fills every free backpack slot with random objects, skipping members-only items
/// (on a free world), dummy items, and cert templates.
///
/// Usage: `::givecrap` (no arguments).
fn cheat_give_crap(active: &mut ActivePlayer) -> Result<(), ScriptError> {
    let objs = &cache().objs;
    let count = objs.count() as i32;
    let world_members = engine().members;
    let rng = &mut engine_mut().random;

    let Some(inv) = cache().invs.get_by_debugname("inv") else {
        return Ok(());
    };
    let Some(inventory) = active.player.invs.get_mut(&inv.id) else {
        return Ok(());
    };

    for _ in 0..inventory.capacity {
        loop {
            let Some(obj) = objs.get_by_id(rng.next_int_bound(count) as u16) else {
                continue;
            };
            if (!world_members && obj.members)
                || obj.dummyitem != DummyItem::None
                || obj.certtemplate.is_some()
            {
                continue;
            }
            inventory.add(obj.id, 1, obj.stackable);
            break;
        }
    }

    Ok(())
}

/// Sets a player variable (varp). For a protected varp, first closes any open modal
/// and clears interaction, aborting with a message if the player is busy.
///
/// Usage: `::setvar <varp> <value>` (e.g. `::setvar testvar 5`).
fn cheat_setvar(args: &mut Split<char>, active: &mut ActivePlayer) -> Result<(), ScriptError> {
    parse_varp(args.next(), |v| {
        if v.protect {
            if let Err(e) = active.close_modal(true) {
                error!("error closing modal during setvar: {e}");
            }
            if !active.can_access() {
                active.message_game("Please finish what you are doing first.");
                return;
            }
            active.player.clear_interaction();
            active.unset_map_flag();
        }
        let value = build_var_value(v, args);
        active.message_game(&format!("Set {:?}: to {:?}", v.debugname(), value));
        active.set_varp(v.id, value, v.transmit);
    })
}

/// Builds a [`VarValue`] for `v` from the next argument token: a raw string for
/// string varps, otherwise an integer (defaulting to `0` when missing or
/// unparseable).
fn build_var_value(v: &VarPlayerType, args: &mut Split<char>) -> VarValue {
    if v.var_type == ScriptVarType::String {
        VarValue::String(args.next().unwrap_or("0").into())
    } else {
        VarValue::from_int(
            v.var_type,
            args.next().unwrap_or("0").parse::<i32>().unwrap_or(0),
        )
    }
}

/// Reports a player variable (varp) back to the player as a game message.
///
/// Usage: `::getvar <varp>` (e.g. `::getvar testvar`).
fn cheat_getvar(args: &mut Split<char>, active: &mut ActivePlayer) -> Result<(), ScriptError> {
    parse_varp(args.next(), |v| {
        let value = build_var_value(v, args);
        active.message_game(&format!("Get {:?}: {:?}", v.debugname(), value));
    })
}

/// Changes the engine clock rate -- the tick length in milliseconds (default 600).
///
/// Usage: `::speed <ms>` (e.g. `::speed 100` to fast-forward, `::speed 600` for normal).
fn cheat_engine_speed(
    args: &mut Split<char>,
    active: &mut ActivePlayer,
) -> Result<(), ScriptError> {
    if let Ok(speed) = args.next().unwrap_or("600").parse::<u64>() {
        engine().set_clock_rate(speed);
        active.message_game(&format!("Engine clock rate changed to: {}ms", speed));
    }
    Ok(())
}

/// Reboots the game world immediately.
///
/// Usage: `::reboot` (no arguments).
fn cheat_reboot() -> Result<(), ScriptError> {
    engine_mut().reboot_timer(0);
    Ok(())
}

/// Reboots the game world after `<seconds>` (default 30), giving players a
/// visible countdown.
///
/// Usage: `::slowreboot <seconds>` (e.g. `::slowreboot 60`).
fn cheat_slow_reboot(args: &mut Split<char>) -> Result<(), ScriptError> {
    let Some(arg) = args.next() else {
        return Ok(());
    };
    let seconds = arg.parse::<i64>().unwrap_or(30).max(0) as u64;
    let clocks = seconds.saturating_mul(1000).saturating_add(599) / 600;
    engine_mut().reboot_timer(clocks);
    Ok(())
}

/// Spawns up to 2000 bot players in a grid centered near (3222, 3222).
///
/// Usage: `::bots` (no arguments).
fn cheat_spawn_bots(_player: &ActivePlayer) -> Result<(), ScriptError> {
    let center_x: i32 = 3222;
    let center_z: i32 = 3222;
    let grid_size: i32 = 45; // 45x45 grid = 2025 slots, enough for 2000
    let half: i32 = grid_size / 2;

    let engine = engine_mut();

    let mut spawned = 0;
    for dx in -half..=half {
        for dz in -half..=half {
            if spawned >= 2000 {
                break;
            }

            let x = (center_x + dx) as u16;
            let z = (center_z + dz) as u16;

            let Some(pid) = engine.player_list.next_pid() else {
                break;
            };

            let isaac = IsaacPair::new(&[0; 4], &[0; 4]);
            let io = create_io(isaac);
            let username: Box<str> = format!("Bot {}", pid).into();
            let mut bot = ActivePlayer::new(io.handle, pid, username, false, true);

            bot.player.pathing.coord = CoordGrid::new(x, 0, z);
            bot.player.pathing.last_coord = bot.player.pathing.coord;
            bot.player.pathing.tele = true;
            bot.player.pathing.jump = true;

            bot.player.info.readyanim = Some(808);
            bot.player.info.turnanim = Some(823);
            bot.player.info.walkanim = Some(819);
            bot.player.info.walkanim_b = Some(820);
            bot.player.info.walkanim_l = Some(821);
            bot.player.info.walkanim_r = Some(822);
            bot.player.info.runanim = Some(824);

            bot.buildappearance(0);

            engine.add_player(pid, bot, pid as i64);
            spawned += 1;
        }
        if spawned >= 2000 {
            break;
        }
    }

    tracing::info!(
        "Spawned {} bots around ({}, {}, 0)",
        spawned,
        center_x,
        center_z
    );
    Ok(())
}

/// Sets the player's visibility -- `0` = default, `1` = soft-hide, `2` = hard-hide.
/// Any other or missing value is ignored.
///
/// Usage: `::setvis <level>` (e.g. `::setvis 2`).
fn cheat_setvis(args: &mut Split<char>, active: &mut ActivePlayer) -> Result<(), ScriptError> {
    active.player.info.vis = match args.next() {
        Some("0") => Visibility::Default,
        Some("1") => Visibility::Soft,
        Some("2") => Visibility::Hard,
        _ => return Ok(()),
    };
    Ok(())
}

/// Reports the player's current coordinate as `level,mx,mz,lx,lz` (level,
/// map-square X/Z, local tile X/Z) -- the same format accepted by `tele`.
///
/// Usage: `::getcoord` (no arguments).
fn cheat_getcoord(active: &mut ActivePlayer) -> Result<(), ScriptError> {
    let coord = active.player.pathing.coord;
    let (x, z) = (coord.x(), coord.z());
    active.message_game(&format!(
        "{},{},{},{},{}",
        coord.y(),
        x >> 6,
        z >> 6,
        x & 0x3f,
        z & 0x3f
    ));
    Ok(())
}

/// Teleports the player to a coordinate given as `level,mx,mz[,lx,lz]` (level,
/// map-square X/Z, local tile X/Z; local tiles default to 32). Requires at least
/// the first three components, all within range (level 0-3, map square 0-255,
/// local tile 0-63); out-of-range or under-specified input is ignored.
///
/// Closes any open modal and clears interaction first, aborting if the player is
/// busy -- mirroring `setvar` on a protected variable.
///
/// Usage: `::tele <level,mx,mz[,lx,lz]>` (e.g. `::tele 0,50,50,32,32`).
fn cheat_tele(args: &mut Split<char>, active: &mut ActivePlayer) -> Result<(), ScriptError> {
    let Some(arg) = args.next() else {
        return Ok(());
    };
    let parts: Vec<&str> = arg.split(',').collect();
    if parts.len() < 3 {
        return Ok(());
    }

    if let Err(e) = active.close_modal(true) {
        error!("error closing modal during tele: {e}");
    }
    if !active.can_access() {
        active.message_game("Please finish what you are doing first.");
        return Ok(());
    }
    active.player.clear_interaction();
    active.unset_map_flag();

    let part = |i: usize, default: i32| {
        parts
            .get(i)
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(default)
    };
    let level = part(0, 0);
    let mx = part(1, 50);
    let mz = part(2, 50);
    let lx = part(3, 32);
    let lz = part(4, 32);

    if !(0..=3).contains(&level)
        || !(0..=255).contains(&mx)
        || !(0..=255).contains(&mz)
        || !(0..=63).contains(&lx)
        || !(0..=63).contains(&lz)
    {
        return Ok(());
    }

    let x = ((mx << 6) + lx) as u16;
    let z = ((mz << 6) + lz) as u16;
    active.tele_jump(CoordGrid::new(x, level as u8, z));
    Ok(())
}

/// Looks up an [`ObjType`] by its debug name and invokes the callback with it.
///
/// # Arguments
///
/// * `value` - The debug name string to look up, or `None` if no argument was provided.
/// * `callback` - Invoked with the resolved `ObjType` reference on success.
///
/// # Returns
///
/// * `Ok(())` if the object was found and the callback executed.
/// * `Err(ScriptError::ObjNotFoundName)` if the debug name is missing or not found in the cache.
fn parse_obj<F>(value: Option<&str>, callback: F) -> Result<(), ScriptError>
where
    F: FnOnce(&ObjType),
{
    if let Some(debugname) = value
        && let Some(obj) = cache().objs.get_by_debugname(debugname)
    {
        callback(obj);
        Ok(())
    } else {
        Err(ScriptError::ObjNotFoundName(
            value.unwrap_or_default().into(),
        ))
    }
}

/// Looks up an [`EnumType`] by its debug name and invokes the callback with it.
///
/// # Arguments
///
/// * `value` - The debug name string to look up, or `None` if no argument was provided.
/// * `callback` - Invoked with the resolved `EnumType` reference on success.
///
/// # Returns
///
/// * `Ok(())` if the enum was found and the callback executed.
/// * `Err(ScriptError::EnumNotFoundName)` if the debug name is missing or not found in the cache.
fn parse_enum<F>(value: Option<&str>, callback: F) -> Result<(), ScriptError>
where
    F: FnOnce(&EnumType),
{
    if let Some(debugname) = value
        && let Some(e) = cache().enums.get_by_debugname(debugname)
    {
        callback(e);
        Ok(())
    } else {
        Err(ScriptError::EnumNotFoundName(
            value.unwrap_or_default().into(),
        ))
    }
}

/// Looks up a [`LocType`] by its debug name and invokes the callback with it.
///
/// # Arguments
///
/// * `value` - The debug name string to look up, or `None` if no argument was provided.
/// * `callback` - Invoked with the resolved `LocType` reference on success.
///
/// # Returns
///
/// * `Ok(())` if the location type was found and the callback executed.
/// * `Err(ScriptError::LocNotFoundName)` if the debug name is missing or not found in the cache.
fn parse_loc<F>(value: Option<&str>, callback: F) -> Result<(), ScriptError>
where
    F: FnOnce(&LocType),
{
    if let Some(debugname) = value
        && let Some(loc) = cache().locs.get_by_debugname(debugname)
    {
        callback(loc);
        Ok(())
    } else {
        Err(ScriptError::LocNotFoundName(
            value.unwrap_or_default().into(),
        ))
    }
}

/// Looks up an [`IfType`] (interface type) by its debug name and invokes the callback with it.
///
/// # Arguments
///
/// * `value` - The debug name string to look up, or `None` if no argument was provided.
/// * `callback` - Invoked with the resolved `IfType` reference on success.
///
/// # Returns
///
/// * `Ok(())` if the interface was found and the callback executed.
/// * `Err(ScriptError::InterfaceNotFoundName)` if the debug name is missing or not found in the cache.
fn parse_interface<F>(value: Option<&str>, callback: F) -> Result<(), ScriptError>
where
    F: FnOnce(&IfType),
{
    if let Some(debugname) = value
        && let Some(interface) = cache().interfaces.get_by_debugname(debugname)
    {
        callback(interface);
        Ok(())
    } else {
        Err(ScriptError::InterfaceNotFoundName(
            value.unwrap_or_default().into(),
        ))
    }
}

/// Looks up a [`SpotAnimType`] by its debug name and invokes the callback with it.
///
/// # Arguments
///
/// * `value` - The debug name string to look up, or `None` if no argument was provided.
/// * `callback` - Invoked with the resolved `SpotAnimType` reference on success.
///
/// # Returns
///
/// * `Ok(())` if the spot animation was found and the callback executed.
/// * `Err(ScriptError::SpotanimNotFoundName)` if the debug name is missing or not found in the cache.
fn parse_spotanim<F>(value: Option<&str>, callback: F) -> Result<(), ScriptError>
where
    F: FnOnce(&SpotAnimType),
{
    if let Some(debugname) = value
        && let Some(spotanim) = cache().spotanims.get_by_debugname(debugname)
    {
        callback(spotanim);
        Ok(())
    } else {
        Err(ScriptError::SpotanimNotFoundName(
            value.unwrap_or_default().into(),
        ))
    }
}

/// Looks up an [`NpcType`] by its debug name and invokes the callback with it.
///
/// # Arguments
///
/// * `value` - The debug name string to look up, or `None` if no argument was provided.
/// * `callback` - Invoked with the resolved `NpcType` reference on success.
///
/// # Returns
///
/// * `Ok(())` if the NPC type was found and the callback executed.
/// * `Err(ScriptError::NpcNotFoundName)` if the debug name is missing or not found in the cache.
fn parse_npc<F>(value: Option<&str>, callback: F) -> Result<(), ScriptError>
where
    F: FnOnce(&NpcType),
{
    if let Some(debugname) = value
        && let Some(npc) = cache().npcs.get_by_debugname(debugname)
    {
        callback(npc);
        Ok(())
    } else {
        Err(ScriptError::NpcNotFoundName(
            value.unwrap_or_default().into(),
        ))
    }
}

/// Looks up an [`InvType`] by its debug name and invokes the callback with it.
///
/// # Arguments
///
/// * `value` - The debug name string to look up, or `None` if no argument was provided.
/// * `callback` - Invoked with the resolved `InvType` reference on success.
///
/// # Returns
///
/// * `Ok(())` if the inventory type was found and the callback executed.
/// * `Err(ScriptError::InvNotFoundName)` if the debug name is missing or not found in the cache.
fn parse_inv<F>(value: Option<&str>, callback: F) -> Result<(), ScriptError>
where
    F: FnOnce(&InvType),
{
    if let Some(debugname) = value
        && let Some(inv) = cache().invs.get_by_debugname(debugname)
    {
        callback(inv);
        Ok(())
    } else {
        Err(ScriptError::InvNotFoundName(
            value.unwrap_or_default().into(),
        ))
    }
}

/// Looks up a [`SeqType`] (animation sequence) by its debug name and invokes the callback with it.
///
/// # Arguments
///
/// * `value` - The debug name string to look up, or `None` if no argument was provided.
/// * `callback` - Invoked with the resolved `SeqType` reference on success.
///
/// # Returns
///
/// * `Ok(())` if the sequence was found and the callback executed.
/// * `Err(ScriptError::SeqNotFoundName)` if the debug name is missing or not found in the cache.
fn parse_seq<F>(value: Option<&str>, callback: F) -> Result<(), ScriptError>
where
    F: FnOnce(&SeqType),
{
    if let Some(debugname) = value
        && let Some(seq) = cache().seqs.get_by_debugname(debugname)
    {
        callback(seq);
        Ok(())
    } else {
        Err(ScriptError::SeqNotFoundName(
            value.unwrap_or_default().into(),
        ))
    }
}

/// Looks up an [`IdkType`] (identity kit type) by its debug name and invokes the callback with it.
///
/// # Arguments
///
/// * `value` - The debug name string to look up, or `None` if no argument was provided.
/// * `callback` - Invoked with the resolved `IdkType` reference on success.
///
/// # Returns
///
/// * `Ok(())` if the identity kit was found and the callback executed.
/// * `Err(ScriptError::IdkNotFoundName)` if the debug name is missing or not found in the cache.
fn parse_idk<F>(value: Option<&str>, callback: F) -> Result<(), ScriptError>
where
    F: FnOnce(&IdkType),
{
    if let Some(debugname) = value
        && let Some(idk) = cache().idks.get_by_debugname(debugname)
    {
        callback(idk);
        Ok(())
    } else {
        Err(ScriptError::IdkNotFoundName(
            value.unwrap_or_default().into(),
        ))
    }
}

/// Looks up a [`VarPlayerType`] by its debug name and invokes the callback with it.
///
/// # Arguments
///
/// * `value` - The debug name string to look up, or `None` if no argument was provided.
/// * `callback` - Invoked with the resolved `VarPlayerType` reference on success.
///
/// # Returns
///
/// * `Ok(())` if the player variable type was found and the callback executed.
/// * `Err(ScriptError::VarpNotFoundName)` if the debug name is missing or not found in the cache.
fn parse_varp<F>(value: Option<&str>, callback: F) -> Result<(), ScriptError>
where
    F: FnOnce(&VarPlayerType),
{
    if let Some(debugname) = value
        && let Some(idk) = cache().varps.get_by_debugname(debugname)
    {
        callback(idk);
        Ok(())
    } else {
        Err(ScriptError::VarpNotFoundName(
            value.unwrap_or_default().into(),
        ))
    }
}

/// Looks up a [`StructType`] by its debug name and invokes the callback with it.
///
/// # Arguments
///
/// * `value` - The debug name string to look up, or `None` if no argument was provided.
/// * `callback` - Invoked with the resolved `StructType` reference on success.
///
/// # Returns
///
/// * `Ok(())` if the struct type was found and the callback executed.
/// * `Err(ScriptError::StructNotFoundName)` if the debug name is missing or not found in the cache.
fn parse_struct<F>(value: Option<&str>, callback: F) -> Result<(), ScriptError>
where
    F: FnOnce(&StructType),
{
    if let Some(debugname) = value
        && let Some(s) = cache().structs.get_by_debugname(debugname)
    {
        callback(s);
        Ok(())
    } else {
        Err(ScriptError::StructNotFoundName(
            value.unwrap_or_default().into(),
        ))
    }
}

/// Looks up a [`CategoryType`] by its debug name and invokes the callback with it.
///
/// # Arguments
///
/// * `value` - The debug name string to look up, or `None` if no argument was provided.
/// * `callback` - Invoked with the resolved `CategoryType` reference on success.
///
/// # Returns
///
/// * `Ok(())` if the category was found and the callback executed.
/// * `Err(ScriptError::CategoryNotFoundName)` if the debug name is missing or not found in the cache.
fn parse_category<F>(value: Option<&str>, callback: F) -> Result<(), ScriptError>
where
    F: FnOnce(&CategoryType),
{
    if let Some(debugname) = value
        && let Some(category) = cache().categories.get_by_debugname(debugname)
    {
        callback(category);
        Ok(())
    } else {
        Err(ScriptError::CategoryNotFoundName(
            value.unwrap_or_default().into(),
        ))
    }
}

/// Parses a boolean value from a string and invokes the callback with the result.
///
/// Accepts `"true"`, `"1"`, `"yes"` as truthy and `"false"`, `"0"`, `"no"` as falsy.
///
/// # Arguments
///
/// * `value` - The string to parse, or `None` if no argument was provided.
/// * `callback` - Invoked with the parsed boolean value on success.
///
/// # Returns
///
/// * `Ok(())` if the value was successfully parsed as a boolean.
/// * `Err(ScriptError::Client)` if the value is not a recognized boolean string.
/// * `Err(ScriptError::BooleanNotFoundName)` if no value was provided.
fn parse_bool<F>(value: Option<&str>, callback: F) -> Result<(), ScriptError>
where
    F: FnOnce(bool),
{
    if let Some(value) = value {
        if value != "true"
            && value != "false"
            && value != "0"
            && value != "1"
            && value != "yes"
            && value != "no"
        {
            return Err(ScriptError::Client(format!("Invalid boolean: {}", value)));
        }
        callback(value == "yes" || value == "true" || value == "1");
        Ok(())
    } else {
        Err(ScriptError::BooleanNotFoundName(
            value.unwrap_or_default().into(),
        ))
    }
}

/// Parses a coordinate string in `y_mx_mz_lx_lz` format and invokes the callback
/// with the resulting [`CoordGrid`].
///
/// The format uses underscore-separated components:
/// - `y` - Height level (0-3)
/// - `mx` - Map square X (0-255)
/// - `mz` - Map square Z (0-255)
/// - `lx` - Local X within map square (0-63)
/// - `lz` - Local Z within map square (0-63)
///
/// # Arguments
///
/// * `value` - The coordinate string to parse, or `None` if no argument was provided.
/// * `callback` - Invoked with the constructed `CoordGrid` on success.
///
/// # Returns
///
/// * `Ok(())` if the coordinate was successfully parsed and constructed.
/// * `Err(ScriptError::Client)` if the format is invalid or values are out of range.
/// * `Err(ScriptError::IdkNotFoundName)` if no value was provided.
fn parse_coord<F>(value: Option<&str>, callback: F) -> Result<(), ScriptError>
where
    F: FnOnce(CoordGrid),
{
    if let Some(coord) = value {
        let err = || ScriptError::Client(format!("Cannot parse coord: {}", coord));
        let parts: Vec<&str> = coord.split('_').collect();
        if parts.len() != 5 {
            return Err(err());
        }
        let y: i32 = parts[0].parse().map_err(|_| err())?;
        let mx: i32 = parts[1].parse().map_err(|_| err())?;
        let mz: i32 = parts[2].parse().map_err(|_| err())?;
        let lx: i32 = parts[3].parse().map_err(|_| err())?;
        let lz: i32 = parts[4].parse().map_err(|_| err())?;

        if !(0..=63).contains(&lz)
            || !(0..=63).contains(&lx)
            || !(0..=255).contains(&mz)
            || !(0..=255).contains(&mx)
            || !(0..=3).contains(&y)
        {
            return Err(err());
        }

        let x = (mx << 6) + lx;
        let z = (mz << 6) + lz;
        callback(CoordGrid::from((z | (x << 14) | (y << 28)) as u32));
        Ok(())
    } else {
        Err(ScriptError::IdkNotFoundName(
            value.unwrap_or_default().into(),
        ))
    }
}

/// Parses a player stat name string into a [`PlayerStat`] and invokes the callback with it.
///
/// Uses `PlayerStat::from_config_str` with panic-catching to safely handle invalid names.
///
/// # Arguments
///
/// * `value` - The stat config string to parse, or `None` if no argument was provided.
/// * `callback` - Invoked with the resolved `PlayerStat` on success.
///
/// # Returns
///
/// * `Ok(())` if the stat was successfully parsed.
/// * `Err(ScriptError::StatNotFoundName)` if the name is invalid or not provided.
fn parse_stat<F>(value: Option<&str>, callback: F) -> Result<(), ScriptError>
where
    F: FnOnce(PlayerStat),
{
    if let Some(stat) = value {
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            PlayerStat::from_config_str(stat)
        }));

        match result {
            Ok(stat) => {
                callback(stat);
                Ok(())
            }
            Err(_) => Err(ScriptError::StatNotFoundName(
                value.unwrap_or_default().into(),
            )),
        }
    } else {
        Err(ScriptError::StatNotFoundName(
            value.unwrap_or_default().into(),
        ))
    }
}

/// Parses an NPC stat name string into an [`NpcStat`] and invokes the callback with it.
///
/// Uses `NpcStat::from_config_str` with panic-catching to safely handle invalid names.
///
/// # Arguments
///
/// * `value` - The NPC stat config string to parse, or `None` if no argument was provided.
/// * `callback` - Invoked with the resolved `NpcStat` on success.
///
/// # Returns
///
/// * `Ok(())` if the stat was successfully parsed.
/// * `Err(ScriptError::NpcstatNotFoundName)` if the name is invalid or not provided.
fn parse_npcstat<F>(value: Option<&str>, callback: F) -> Result<(), ScriptError>
where
    F: FnOnce(NpcStat),
{
    if let Some(stat) = value {
        let result =
            panic::catch_unwind(panic::AssertUnwindSafe(|| NpcStat::from_config_str(stat)));

        match result {
            Ok(stat) => {
                callback(stat);
                Ok(())
            }
            Err(_) => Err(ScriptError::NpcstatNotFoundName(
                value.unwrap_or_default().into(),
            )),
        }
    } else {
        Err(ScriptError::NpcstatNotFoundName(
            value.unwrap_or_default().into(),
        ))
    }
}
