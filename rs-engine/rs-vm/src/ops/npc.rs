use crate::engine::{ScriptEngine, ScriptNpc, cache, engine, engine_mut};
use crate::iterators::NpcIteratorState;
use crate::register::OpsRegistry;
use crate::state::{ExecutionState, ScriptArgument};
use crate::trigger::ServerTriggerType;
use crate::util::*;
use crate::{NpcUid, ScriptError, active_npc, handlers, none};
use crate::{active_npc_mut, iterators};
use rs_grid::CoordGrid;
use rs_pack::ParamValue;
use rs_pack::cache::script::*;
use rs_pack::types::{HuntCheckVis, NpcMode};

/// Registers NPC-related opcodes for spawning, movement, combat, AI mode
/// management, searching, and NPC state queries.
///
/// # Opcodes Registered
///
/// Key opcodes include:
/// - **Lifecycle:** `NPC_ADD`, `NPC_DEL`, `NPC_CHANGETYPE`, `NPC_CHANGETYPE_KEEPALL`
/// - **Identity / state:** `NPC_UID`, `NPC_TYPE`, `NPC_NAME`, `NPC_CATEGORY`,
///   `NPC_COORD`, `NPC_STAT`, `NPC_HASOP`, `NPC_GETMODE`, `NPC_RANGE`
/// - **Movement:** `NPC_WALK`, `NPC_TELE`, `NPC_FACESQUARE`, `NPC_ARRIVEDELAY`
/// - **Combat:** `NPC_ANIM`, `NPC_DAMAGE`, `NPC_HEROPOINTS`, `NPC_SAY`, `PROJANIM_NPC`
/// - **AI / behavior:** `NPC_SETMODE`, `NPC_SETHUNT`, `NPC_SETHUNTMODE`,
///   `NPC_SETTIMER`, `NPC_QUEUE`
/// - **Searching / iterators:** `NPC_FIND`, `NPC_FINDALLANY`, `NPC_FINDNEXT`,
///   `NPC_FINDUID`, `NPC_FINDHERO`, `NPC_HUNT`, `NPC_HUNTALL`
/// - **Params:** `NPC_PARAM`
/// - **Timing:** `NPC_DELAY`
///
/// # Call Stack
///
/// **Called by:** `Engine::new` (in `rs-engine/src/engine.rs`) via `ops::npc::build`
/// **Calls:** `OpsRegistry::new`, `OpsRegistry::insert` via the `handlers!` / `active_npc!` /
/// `active_npc_mut!` / `none!` macros
pub fn build<E: ScriptEngine + 'static>() -> OpsRegistry {
    handlers! { |m|
        // 2500
        none!(m, NPC_ADD => |s| {
            let duration = s.pop_int();
            let id = s.pop_int_as::<u16>()?;
            let coord = s.pop_int();
            if let Some(uid) = engine_mut::<E>().add_npc_spawned(coord as u32, id, duration as u64) {
                set_active_npc(s, uid, s.int_operand() != 0);
            }
        });

        // 2501
        active_npc_mut!(m, NPC_ANIM => |s, npc| {
            let delay = s.pop_int_as::<u8>()?;
            let seq = s.pop_int();
            npc.anim((seq != -1).then_some(seq as u16), delay);
        });

        // 2502
        // https://x.com/JagexAsh/status/1432296606376906752
        active_npc_mut!(m, NPC_ARRIVEDELAY => |s, npc| {
            let clock = engine::<E>().clock();
            if npc.last_movement() < clock.saturating_sub(1) {
                return Ok(());
            }
            if npc.last_movement() == clock - 1 {
                npc.delay(clock + 1);
            } else {
                npc.delay(clock + 2);
            }
            s.execution = ExecutionState::NpcSuspended;
        });

        // 2503
        // https://twitter.com/JagexAsh/status/1614498680144527360
        active_npc_mut!(m, NPC_ATTACKRANGE => |s, npc| {
            let npc_type = cache()
                .npcs
                .get_by_id(npc.uid().id())
                .ok_or(ScriptError::NpcNotFound(npc.uid().id() as i32))?;
            s.push_int(npc_type.attackrange as i32);
        });

        // 2504
        active_npc_mut!(m, NPC_BASESTAT => |s, npc| {
            let stat = s.pop_int() as usize;
            s.push_int(npc.basestat(stat) as i32);
        });

        // 2505
        active_npc!(m, NPC_CATEGORY => |s, npc| {
            let npc_type = cache()
                .npcs
                .get_by_id(npc.uid().id())
                .ok_or(ScriptError::NpcNotFound(npc.uid().id() as i32))?;
            s.push_int(npc_type.category.map(|c| c as i32).unwrap_or(-1));
        });

        // 2506
        active_npc_mut!(m, NPC_CHANGETYPE_KEEPALL => |s, npc| {
            let duration = s.pop_int();
            let npc_type = pop_npc(s)?;
            npc.change_type(npc_type.id, duration as u64, false);
        });

        // 2507
        active_npc_mut!(m, NPC_CHANGETYPE => |s, npc| {
            let duration = s.pop_int();
            let npc_type = pop_npc(s)?;
            npc.change_type(npc_type.id, duration as u64, true);
        });

        // 2508
        // https://x.com/JagexAsh/status/1821835323808026853
        active_npc!(m, NPC_COORD => |s, npc| {
            s.push_int(npc.coord() as i32);
        });

        // 2509
        active_npc_mut!(m, NPC_DAMAGE => |s, npc| {
            let amount = s.pop_int_as::<u8>()?;
            let damage_type = s.pop_int_as::<u8>()?;
            npc.damage(amount, damage_type);
        });

        // 2510
        active_npc!(m, NPC_DEL => |s, npc| {
            engine_mut::<E>().remove_npc(npc.uid().nid());
        });

        // 2511
        active_npc_mut!(m, NPC_DELAY => |s, npc| {
            let delay = s.pop_int();
            let clock = engine::<E>().clock();
            npc.delay(clock + 1 + delay as u32);
            s.execution = ExecutionState::NpcSuspended;
        });

        // 2512
        active_npc_mut!(m, NPC_FACESQUARE => |s, npc| {
            let coord = CoordGrid::from(s.pop_int() as u32);
            npc.facesquare(coord.x(), coord.z());
        });

        // 2513
        // https://x.com/JagexAsh/status/1796460129430433930
        none!(m, NPC_FIND => |s| {
            let vis = HuntCheckVis::try_from(s.pop_int() as u8).unwrap_or(HuntCheckVis::Off);
            let distance = s.pop_int();
            let npc = pop_npc(s)?;
            let coord = CoordGrid::from(s.pop_int() as u32);

            let npcs = iterators::npc_distance::<E>(npc.id, coord, distance, vis);

            let closest = npcs
                .iter()
                .min_by_key(|r| coord.euclidean_squared_distance(CoordGrid::from(r.coord)));

            if let Some(npc_ref) = closest {
                set_active_npc(s, NpcUid::new(npc_ref.id, npc_ref.nid), s.int_operand() != 0);
                s.push_int(1);
            } else {
                s.push_int(0);
            }
        });

        // 2514
        none!(m, NPC_FINDALL => |s| {
            let vis = HuntCheckVis::try_from(s.pop_int() as u8).unwrap_or(HuntCheckVis::Off);
            let distance = s.pop_int();
            let npc = pop_npc(s)?;
            let coord = CoordGrid::from(s.pop_int() as u32);
            let npcs = iterators::npc_distance::<E>(npc.id, coord, distance, vis);
            s.npc_iterator = Some(NpcIteratorState {
                matches: npcs,
                cursor: 0,
            });
        });

        // 2515
        // https://x.com/JagexAsh/status/1796878374398246990
        none!(m, NPC_FINDALLANY => |s| {
            let vis = HuntCheckVis::try_from(s.pop_int() as u8).unwrap_or(HuntCheckVis::Off);
            let distance = s.pop_int();
            let coord = CoordGrid::from(s.pop_int() as u32);
            let npcs = iterators::npc_distance_any::<E>(coord, distance, vis);
            s.npc_iterator = Some(NpcIteratorState {
                matches: npcs,
                cursor: 0,
            });
        });

        // 2516
        none!(m, NPC_FINDALLZONE => |s| {
            let coord = CoordGrid::from(s.pop_int() as u32);
            let npcs = iterators::npc_zone::<E>(coord);
            s.npc_iterator = Some(NpcIteratorState {
                matches: npcs,
                cursor: 0,
            });
        });

        // 2517
        none!(m, NPC_FINDCAT => |s| {
            let vis = HuntCheckVis::try_from(s.pop_int() as u8).unwrap_or(HuntCheckVis::Off);
            let distance = s.pop_int();
            let category = s.pop_int();
            let coord = CoordGrid::from(s.pop_int() as u32);

            let npcs = iterators::npc_distance_any::<E>(coord, distance, vis);
            let c = cache();

            let closest = npcs
                .iter()
                .filter(|r| {
                    c.npcs
                        .get_by_id(r.id)
                        .and_then(|t| t.category)
                        .is_some_and(|cat| cat as i32 == category)
                })
                .min_by_key(|r| coord.euclidean_squared_distance(CoordGrid::from(r.coord)));

            if let Some(npc_ref) = closest {
                set_active_npc(s, NpcUid::new(npc_ref.id, npc_ref.nid), s.int_operand() != 0);
                s.push_int(1);
            } else {
                s.push_int(0);
            }
        });

        // 2518
        none!(m, NPC_FINDEXACT => |s| {
            let id = s.pop_int() as u16;
            let coord = CoordGrid::from(s.pop_int() as u32);

            let npcs = iterators::npc_zone::<E>(coord);

            if let Some(npc_ref) = npcs.iter().find(|r| {
                let c = CoordGrid::from(r.coord);
                r.id == id && c.x() == coord.x() && c.z() == coord.z() && c.y() == coord.y()
            }) {
                set_active_npc(s, NpcUid::new(npc_ref.id, npc_ref.nid), s.int_operand() != 0);
                s.push_int(1);
            } else {
                s.push_int(0);
            }

            s.npc_iterator = Some(NpcIteratorState {
                matches: npcs,
                cursor: 0,
            });
        });

        // 2519
        active_npc!(m, NPC_FINDHERO => |s, npc| {
            let Some(user37) = npc.findhero() else {
                s.push_int(0);
                return Ok(());
            };
            let Some(uid) = engine::<E>().find_player_by_user37(user37) else {
                s.push_int(0);
                return Ok(());
            };
            set_active_player(s, uid, s.int_operand() != 0);
            s.push_int(1);
        });

        // 2520
        none!(m, NPC_FINDNEXT => |s| {
            let iter = match s.npc_iterator.as_mut() {
                Some(iter) => iter,
                None => {
                    s.push_int(0);
                    return Ok(());
                }
            };
            if iter.cursor < iter.matches.len() {
                let npc_ref = iter.matches[iter.cursor];
                iter.cursor += 1;
                set_active_npc(s, NpcUid::new(npc_ref.id, npc_ref.nid), s.int_operand() != 0);
                s.push_int(1);
            } else {
                s.push_int(0);
            }
        });

        // 2521
        none!(m, NPC_FINDUID => |s| {
            let uid = s.pop_int();
            let nid = (uid & 0xFFFF) as u16;
            match engine::<E>().get_npc(nid) {
                None => s.push_int(0),
                Some(npc) => {
                    set_active_npc(s, npc.uid(), s.int_operand() != 0);
                    s.push_int(1);
                }
            }
        });

        // 2522
        active_npc!(m, NPC_GETMODE => |s, npc| {
            s.push_int(npc.target_op().map(|x| x as i32).unwrap_or(-1));
        });

        // 2523
        // https://x.com/JagexAsh/status/1821492251429679257
        active_npc!(m, NPC_HASOP => |s, npc| {
            let op = s.pop_int();
            let npc_type = cache()
                .npcs
                .get_by_id(npc.uid().id())
                .ok_or(ScriptError::NpcNotFound(npc.uid().id() as i32))?;
            let has = npc_type
                .op
                .as_ref()
                .and_then(|ops| ops.get((op - 1) as usize))
                .is_some_and(|o| o.is_some());
            s.push_int(has as i32);
        });

        // 2524
        // https://x.com/JagexAsh/status/1704492467226091853
        active_npc_mut!(m, NPC_HEROPOINTS => |s, npc| {
            require_active_player(s)?;
            let points = s.pop_int();
            let player_uid = s.active_player.ok_or(ScriptError::Runtime("no active player".into()))?;
            npc.heropoints(player_uid.username37(), points);
        });

        // 2525
        none!(m, NPC_HUNT => |s| {
            let vis = HuntCheckVis::try_from(s.pop_int() as u8).unwrap_or(HuntCheckVis::Off);
            let distance = s.pop_int();
            let coord = CoordGrid::from(s.pop_int() as u32);

            let npcs = iterators::npc_distance_any::<E>(coord, distance, vis);

            let closest = npcs
                .iter()
                .filter(|npc_ref| {
                    cache()
                        .npcs
                        .get_by_id(npc_ref.id)
                        .and_then(|t| t.op.as_ref())
                        .is_some_and(|ops| ops.get(1).is_some_and(|o| o.is_some()))
                })
                .min_by_key(|r| coord.euclidean_squared_distance(CoordGrid::from(r.coord)));

            if let Some(npc_ref) = closest {
                set_active_npc(s, NpcUid::new(npc_ref.id, npc_ref.nid), s.int_operand() != 0);
                s.push_int(1);
            } else {
                s.push_int(0);
            }
        });

        // 2526
        // https://x.com/JagexAsh/status/1796460129430433930
        // https://x.com/JagexAsh/status/1821236327150710829
        none!(m, NPC_HUNTALL => |s| {
            let vis = HuntCheckVis::try_from(s.pop_int() as u8).unwrap_or(HuntCheckVis::Off);
            let distance = s.pop_int();
            let coord = CoordGrid::from(s.pop_int() as u32);

            let mut npcs = iterators::npc_distance_any::<E>(coord, distance, vis);
            npcs.retain(|npc_ref| {
                cache()
                    .npcs
                    .get_by_id(npc_ref.id)
                    .and_then(|t| t.op.as_ref())
                    .is_some_and(|ops| ops.get(1).is_some_and(|o| o.is_some()))
            });

            s.npc_iterator = Some(NpcIteratorState {
                matches: npcs,
                cursor: 0,
            });
        });

        // 2527
        active_npc!(m, NPC_INRANGE => |s, npc| {
            s.push_int(npc.inrange() as i32);
        });

        // 2528
        active_npc!(m, NPC_NAME => |s, npc| {
            let npc_type = cache()
                .npcs
                .get_by_id(npc.uid().id())
                .ok_or(ScriptError::NpcNotFound(npc.uid().id() as i32))?;
            s.push_string(npc_type.name.as_deref().unwrap_or(npc_type.debugname().unwrap_or("null")));
        });

        // 2529
        active_npc!(m, NPC_PARAM => |s, npc| {
            let param = pop_param(s)?;
            let value = cache()
                .npcs
                .get_by_id(npc.uid().id())
                .ok_or(ScriptError::ParamNotFound(param.id as i32))?
                .params
                .as_ref()
                .and_then(|p| param.get_param_or_default(p))
                .cloned()
                .unwrap_or_else(|| param.default_param());
            match value {
                ParamValue::Int(v) => s.push_int(v),
                ParamValue::String(v) => s.push_string(&v),
            }
        });

        // 2530
        // https://x.com/JagexAsh/status/1570357528172859392
        active_npc_mut!(m, NPC_QUEUE => |s, npc| {
            let delay = s.pop_int_as::<u16>()?;
            let arg = s.pop_int();
            let queue_id = s.pop_int();
            let trigger = ServerTriggerType::AiQueue1 as i32 + queue_id - 1;
            npc.queue(trigger, delay, Some(vec![ScriptArgument::Int(arg)]))?;
        });

        // 2531
        active_npc!(m, NPC_RANGE => |s, npc| {
            let coord = CoordGrid::from(s.pop_int() as u32);
            let npc_coord = CoordGrid::from(npc.coord());
            if coord.y() != npc_coord.y() {
                s.push_int(-1);
            } else {
                let size = npc.size() as i32;
                s.push_int(CoordGrid::distance_to(
                    npc_coord.x() as i32,
                    npc_coord.z() as i32,
                    size,
                    size,
                    coord.x() as i32,
                    coord.z() as i32,
                    1,
                    1,
                ));
            }
        });

        // 2532
        active_npc_mut!(m, NPC_SAY => |s, npc| {
            let msg = s.pop_string();
            npc.say(&msg);
        });

        // 2533
        active_npc_mut!(m, NPC_SETHUNT => |s, npc| {
            npc.set_hunt_range(s.pop_int_as::<u8>()?);
        });

        // 2534
        active_npc_mut!(m, NPC_SETHUNTMODE => |s, npc| {
            let hunt_type_id = s.pop_int();
            if hunt_type_id == -1 {
                npc.set_hunt_mode(None);
            } else {
                npc.set_hunt_mode(Some(hunt_type_id as u16));
            }
        });

        // 2535
        // https://x.com/JagexAsh/status/1795184135327089047
        // https://x.com/JagexAsh/status/1821835323808026853
        active_npc_mut!(m, NPC_SETMODE => |s, npc| {
            let mode = s.pop_int();

            if mode == NpcMode::None as i32
                || mode == NpcMode::Wander as i32
                || mode == NpcMode::Patrol as i32
            {
                npc.clear_interaction();
                npc.set_mode(Some(mode as u8));
                if mode == NpcMode::Patrol as i32 {
                    npc.clear_patrol();
                }
                return Ok(());
            }

            if mode == -1 {
                npc.reset_defaults();
                return Ok(());
            }

            npc.set_mode(Some(mode as u8));

            let operand = s.int_operand();
            let op = mode as u8;
            if mode >= NpcMode::OpNpc1 as i32 {
                let npc_uid = if operand == 0 { s.active_npc2 } else { s.active_npc };
                match npc_uid {
                    Some(uid) => npc.set_interaction_npc(uid.nid(), op),
                    None => npc.reset_defaults(),
                }
            } else if mode >= NpcMode::OpObj1 as i32 {
                match s.active_obj {
                    Some(obj) => npc.set_interaction_obj(obj.coord, obj.id, obj.count, op),
                    None => npc.reset_defaults(),
                }
            } else if mode >= NpcMode::OpLoc1 as i32 {
                match s.active_loc {
                    Some(loc) => {
                        let loc_type = cache().locs.get_by_id(loc.id);
                        let width = loc_type.map(|lt| lt.width).unwrap_or(1);
                        let length = loc_type.map(|lt| lt.length).unwrap_or(1);
                        npc.set_interaction_loc(loc.coord, loc.id, width, length, loc.shape, loc.angle, loc.layer, op);
                    }
                    None => npc.reset_defaults(),
                }
            } else {
                match s.active_player {
                    Some(uid) => npc.set_interaction_player(uid.pid(), op),
                    None => npc.reset_defaults(),
                }
            }
        });

        // 2536
        active_npc_mut!(m, NPC_SETTIMER => |s, npc| {
            let interval = s.pop_int();
            npc.settimer((interval != -1).then_some(interval as u16));
        });

        // 2537
        active_npc!(m, NPC_STAT => |s, npc| {
            let stat = s.pop_int() as usize;
            s.push_int(npc.stat(stat) as i32);
        });

        // 2538
        active_npc_mut!(m, NPC_STATADD => |s, npc| {
            let percent = s.pop_int();
            let constant = s.pop_int();
            let stat = s.pop_int() as usize;
            npc.statadd(stat, constant, percent);
        });

        // 2539
        active_npc_mut!(m, NPC_STATHEAL => |s, npc| {
            let percent = s.pop_int();
            let constant = s.pop_int();
            let stat = s.pop_int() as usize;
            npc.statheal(stat, constant, percent);
        });

        // 2540
        active_npc_mut!(m, NPC_STATSUB => |s, npc| {
            let percent = s.pop_int();
            let constant = s.pop_int();
            let stat = s.pop_int() as usize;
            npc.statsub(stat, constant, percent);
        });

        // 2541
        active_npc_mut!(m, NPC_TELE => |s, npc| {
            let coord = s.pop_int() as u32;
            npc.tele(coord);
        });

        // 2542
        active_npc!(m, NPC_TYPE => |s, npc| {
            s.push_int(npc.uid().id() as i32);
        });

        // 2543
        active_npc!(m, NPC_UID => |s, npc| {
            s.push_int(npc.uid().packed() as i32);
        });

        // 2544
        // https://x.com/JagexAsh/status/1821835323808026853
        // https://x.com/JagexAsh/status/1780932943038345562
        active_npc_mut!(m, NPC_WALK => |s, npc| {
            let coord = CoordGrid::from(s.pop_int() as u32);
            npc.walk(coord.x(), coord.z());
        });

        // 2545
        // https://x.com/JagexAsh/status/1780932943038345562
        active_npc_mut!(m, NPC_WALKTRIGGER => |s, npc| {
            let arg = s.pop_int();
            let queue_id = s.pop_int();
            npc.walktrigger(queue_id - 1, arg);
        });

        // 2546
        active_npc!(m, PROJANIM_NPC => |s, npc| {
            let arc = s.pop_int_as::<u8>()?;
            let peak = s.pop_int_as::<u8>()?;
            let duration = s.pop_int_as::<u16>()?;
            let delay = s.pop_int_as::<u16>()?;
            let dst_height = s.pop_int_as::<u8>()?;
            let src_height = s.pop_int_as::<u8>()?;
            let spotanim = pop_spotanim(s)?;
            let uid = s.pop_int();
            let src = CoordGrid::from(s.pop_int() as u32);
            let dst = CoordGrid::from(npc.coord());
            if uid as u32 != npc.uid().packed() {
                return Err(ScriptError::Runtime(format!("Invalid uid: {}, expected: {}", uid, npc.uid().packed())))
            }
            engine_mut::<E>().map_proj_anim(
                src.y(),
                src.x(),
                src.z(),
                dst.x(),
                dst.z(),
                ((uid & 0xFFFF) as u16).wrapping_add(1) as i16,
                spotanim.id,
                src_height << 2,
                dst_height << 2,
                delay,
                duration,
                peak,
                arc
            );
        });

        // 2547
        active_npc_mut!(m, SPOTANIM_NPC => |s, npc| {
            let delay = s.pop_int_as::<u16>()?;
            let height = s.pop_int_as::<u16>()?;
            let id = s.pop_int_as::<u16>()?;
            npc.spotanim(id, height, delay);
        });

        // 2548
        active_npc!(m, NPC_DESTINATION => |s, npc| {
            s.push_int(npc.destination() as i32);
        });

        // 2549
        // TODO: this is duplicated with `NPC_FINDNEXT`
        none!(m, NPC_HUNTNEXT => |s| {
            let iter = match s.npc_iterator.as_mut() {
                Some(iter) => iter,
                None => {
                    s.push_int(0);
                    return Ok(());
                }
            };
            if iter.cursor < iter.matches.len() {
                let npc_ref = iter.matches[iter.cursor];
                iter.cursor += 1;
                set_active_npc(s, NpcUid::new(npc_ref.id, npc_ref.nid), s.int_operand() != 0);
                s.push_int(1);
            } else {
                s.push_int(0);
            }
        });
    }
}
