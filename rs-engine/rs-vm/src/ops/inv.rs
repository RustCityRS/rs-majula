use crate::engine::{ScriptEngine, ScriptPlayer, engine, engine_mut};
use crate::register::OpsRegistry;
use crate::state::ObjRef;
use crate::state::ScriptState;
use crate::util::*;
use crate::{ScriptError, active_player_mut, handlers, none};
use rs_inv::StackMode;
use rs_pack::cache::inv::InvScope;
use rs_pack::cache::provider::CacheType;
use rs_pack::cache::script::*;
use rs_pack::types::DummyItem;

/// Registers inventory management opcodes for adding, removing, moving,
/// querying, and transmitting item data to the client.
///
/// Most operations enforce protected-access checks for inventories that
/// require it, and handle overflow by dropping excess items on the ground.
///
/// # Opcodes Registered
///
/// Key opcodes include:
/// - **Add / remove:** `INV_ADD`, `INV_DEL`, `INV_DELSLOT`, `INV_CLEAR`, `INV_SETSLOT`
/// - **Movement:** `INV_MOVEITEM`, `INV_MOVEITEM_CERT`, `INV_MOVEITEM_UNCERT`,
///   `INV_MOVEFROMSLOT`, `INV_MOVETOSLOT`
/// - **Queries:** `INV_GETOBJ`, `INV_GETNUM`, `INV_TOTAL`, `INV_TOTALCAT`,
///   `INV_TOTALPARAM`, `INV_TOTALPARAM_STACK`, `INV_FREESPACE`,
///   `INV_ITEMSPACE`, `INV_ITEMSPACE2`, `INV_SIZE`, `INV_STOCKBASE`
/// - **Drops:** `INV_DROPSLOT`, `INV_DROPITEM_DELAYED`
/// - **Client sync:** `INV_TRANSMIT`, `INV_STOPTRANSMIT`
///
/// # Call Stack
///
/// **Called by:** `Engine::new` (in `rs-engine/src/engine.rs`) via `ops::inv::build`
/// **Calls:** `OpsRegistry::new`, `OpsRegistry::insert` via the `handlers!` /
/// `active_player_mut!` / `none!` macros
pub fn build<E: ScriptEngine + 'static>() -> OpsRegistry {
    handlers! { |m|
        // 4300
        active_player_mut!(m, BOTH_DROPSLOT => |s, player| {
            let duration = s.pop_int();
            let slot = s.pop_int();
            let coord = pop_coord(s)?;
            let inv = pop_inv::<E>(s)?;
            let secondary = s.int_operand() as usize;
            require_inv_access(s, inv, secondary)?;
            let to_uid = if secondary != 0 { s.active_player } else { s.active_player2 }
                .ok_or(ScriptError::Runtime("player is null".into()))?;

            let inventory = get_inv_mut::<E>(inv.id, player)?;
            let item = inventory.get(slot as u16).copied()
                .ok_or(ScriptError::Runtime(format!("Slot: {} is empty", slot)))?;
            let obj_type = engine::<E>().objs().get_by_id(item.obj)
                .ok_or_else(|| ScriptError::Runtime(format!("Invalid obj: {}", item.obj)))?;

            let completed = inventory.delete(item.obj, item.num);
            if completed == 0 || !obj_type.tradeable {
                return Ok(());
            }
            engine_mut::<E>().add_obj(coord, item.obj, item.num, Some(to_uid.username37()), duration as u64)?;
        });

        // 4301
        // https://x.com/JagexAsh/status/1681295591639248897
        // https://x.com/JagexAsh/status/1799020087086903511
        active_player_mut!(m, BOTH_MOVEINV => |s, player| {
            let to_inv = pop_inv::<E>(s)?;
            let from_inv = pop_inv::<E>(s)?;
            let secondary = s.int_operand() as usize;
            require_inv_access(s, from_inv, secondary)?;
            if !s.pointers.has(ScriptState::PROTECTED_ACTIVE_PLAYER[if secondary != 0 { 0 } else { 1 }])
                && to_inv.protect
                && from_inv.scope != InvScope::Shared {
                return Err(ScriptError::Runtime(format!("Inv: {:?} requires protected access!", to_inv.debugname())));
            }
            let to_uid = if secondary != 0 { s.active_player } else { s.active_player2 }
                .ok_or(ScriptError::Runtime("player is null".into()))?;
            let to_pid = to_uid.pid();
            let to_receiver37 = Some(to_uid.username37());
            let inventory = get_inv_mut::<E>(from_inv.id, player)?;
            let items: Vec<_> = inventory.slots.iter().filter_map(|s| *s).collect();
            inventory.clear();

            let to_player = engine_mut::<E>().get_player_mut(to_pid)
                .ok_or(ScriptError::Runtime("to player not found".into()))?;
                let to_coord = to_player.coord();

            let objs = engine::<E>().objs();

            for item in items {
                let obj_type = objs.get_by_id(item.obj)
                    .ok_or_else(|| ScriptError::Runtime(format!("Invalid obj: {}", item.obj)))?;
                let overflow = get_inv_mut::<E>(to_inv.id, to_player)?.add(item.obj, item.num, obj_type.stackable);
                if overflow > 0 {
                    add_obj_split::<E>(to_coord, item.obj, overflow, obj_type.stackable, to_receiver37, LOOTDROP_DURATION)?;
                }
            }
        });

        // 4302
        active_player_mut!(m, INV_ADD => |s, player| {
            let count = pop_count(s)?;
            let obj = pop_obj::<E>(s)?;
            let inv = pop_inv::<E>(s)?;
            require_inv_access(s, inv, s.int_operand() as usize)?;
            if !inv.dummyinv && obj.dummyitem != DummyItem::None {
                return Err(ScriptError::Runtime(format!("dummyitem in non-dummyinv: {:?} -> {:?}", obj.debugname(), inv.debugname())));
            }
            let coord = player.coord();
            let obj_id = obj.id;
            let overflow = get_inv_mut::<E>(inv.id, player)?.add(obj_id, count, obj.stackable);
            if overflow > 0 {
                let receiver37 = Some(player.uid().username37());
                add_obj_split::<E>(coord, obj_id, overflow, obj.stackable, receiver37, LOOTDROP_DURATION)?;
            }
        });

        // 4303
        none!(m, INV_ALLSTOCK => |s| {
            let inv = pop_inv::<E>(s)?;
            s.push_int(inv.allstock as i32);
        });

        // 4304
        active_player_mut!(m, INV_CHANGESLOT => |s, player| {
            let replace_count = pop_count(s)?;
            let replace_obj = pop_obj::<E>(s)?;
            let find_obj = pop_obj::<E>(s)?;
            let inv = pop_inv::<E>(s)?;
            require_inv_access(s, inv, s.int_operand() as usize)?;
            let inventory = get_inv_mut::<E>(inv.id, player)?;
            for slot in 0..inventory.capacity {
                if let Some(item) = inventory.get(slot as u16) {
                    if item.obj == find_obj.id {
                        inventory.set(slot as u16, replace_obj.id, replace_count);
                        return Ok(());
                    }
                }
            }
        });

        // 4305
        active_player_mut!(m, INV_CLEAR => |s, player| {
            let inv = pop_inv::<E>(s)?;
            require_inv_access(s, inv, s.int_operand() as usize)?;
            get_inv_mut::<E>(inv.id, player)?.clear();
        });

        // 4306
        none!(m, INV_DEBUGNAME => |s| {
            let inv = pop_inv::<E>(s)?;
            s.push_string(inv.debugname().unwrap_or("null"));
        });

        // 4307
        // https://x.com/JagexAsh/status/1679942100249464833
        // https://x.com/JagexAsh/status/1708084689141895625
        active_player_mut!(m, INV_DEL => |s, player| {
            let count = pop_count(s)?;
            let obj = pop_obj::<E>(s)?;
            let inv = pop_inv::<E>(s)?;
            require_inv_access(s, inv, s.int_operand() as usize)?;
            get_inv_mut::<E>(inv.id, player)?.delete(obj.id, count);
        });

        // 4308
        active_player_mut!(m, INV_DELSLOT => |s, player| {
            let slot = s.pop_int();
            let inv = pop_inv::<E>(s)?;
            require_inv_access(s, inv, s.int_operand() as usize)?;
            get_inv_mut::<E>(inv.id, player)?.delete_slot(slot as u16);
        });

        // 4309
        // https://x.com/JagexAsh/status/1778879334167548366
        active_player_mut!(m, INV_DROPALL => |s, player| {
            let duration = s.pop_int();
            let coord = pop_coord(s)?;
            let inv = pop_inv::<E>(s)?;
            require_inv_access(s, inv, s.int_operand() as usize)?;
            let inventory = get_inv_mut::<E>(inv.id, player)?;

            let objs = engine::<E>().objs();

            for slot in 0..inventory.capacity {
                let Some(item) = inventory.get(slot as u16).copied() else {
                    continue
                };
                let obj_type = objs.get_by_id(item.obj)
                    .ok_or_else(|| ScriptError::Runtime(format!("Invalid obj: {}", item.obj)))?;
                inventory.delete_slot(slot as u16);
                if !obj_type.tradeable {
                    continue;
                }
                engine_mut::<E>().add_obj(coord, item.obj, item.num, None, duration as u64)?;
            }
        });

        // 4310
        active_player_mut!(m, INV_DROPITEM_DELAYED => |s, player| {
            let delay = s.pop_int();
            let duration = s.pop_int();
            let count = pop_count(s)?;
            let obj = pop_obj::<E>(s)?;
            let coord = pop_coord(s)?;
            let inv = pop_inv::<E>(s)?;
            require_inv_access(s, inv, s.int_operand() as usize)?;
            let completed = get_inv_mut::<E>(inv.id, player)?.delete(obj.id, count);
            if completed == 0 {
                return Ok(());
            }
            engine_mut::<E>().add_obj_delayed(
                coord,
                obj.id,
                completed,
                Some(player.uid().username37()),
                duration as u64,
                delay as u64,
            );
        });

        // 4311
        // https://x.com/JagexAsh/status/1679942100249464833
        active_player_mut!(m, INV_DROPITEM => |s, player| {
            let duration = s.pop_int();
            let count = pop_count(s)?;
            let obj = pop_obj::<E>(s)?;
            let coord = pop_coord(s)?;
            let inv = pop_inv::<E>(s)?;
            let secondary = s.int_operand() as usize;
            require_inv_access(s, inv, secondary)?;
            let completed = get_inv_mut::<E>(inv.id, player)?.delete(obj.id, count);
            if completed == 0 {
                return Ok(());
            }
            engine_mut::<E>().add_obj(
                coord,
                obj.id,
                completed,
                Some(player.uid().username37()),
                duration as u64
            )?;
            set_active_obj(s, ObjRef { coord, id: obj.id, count: completed }, secondary != 0);
        });

        // 4312
        // https://x.com/JagexAsh/status/1679942100249464833
        active_player_mut!(m, INV_DROPSLOT => |s, player| {
            let duration = s.pop_int();
            let slot = s.pop_int();
            let coord = pop_coord(s)?;
            let inv = pop_inv::<E>(s)?;
            let secondary = s.int_operand() as usize;
            require_inv_access(s, inv, secondary)?;

            let inventory = get_inv_mut::<E>(inv.id, player)?;
            let Some(item) = inventory.get(slot as u16).copied() else {
                return Err(ScriptError::Runtime(format!("Slot: {} is empty", slot)));
            };

            let obj_type = engine::<E>()
                .objs()
                .get_by_id(item.obj)
                .ok_or_else(|| ScriptError::Runtime(format!("Invalid obj: {}", item.obj)))?;

            let stackable = obj_type.stackable;

            inventory.remove(slot as u16, item.num);

            let receiver37 = Some(player.uid().username37());
            add_obj_split::<E>(coord, item.obj, item.num, stackable, receiver37, duration as u64)?;
            set_active_obj(s, ObjRef { coord, id: item.obj, count: item.num }, secondary != 0);
        });

        // 4313
        active_player_mut!(m, INV_FREESPACE => |s, player| {
            let inv = pop_inv::<E>(s)?;
            let inventory = get_inv::<E>(inv, player)?;
            s.push_int(inventory.freespace() as i32);
        });

        // 4314
        active_player_mut!(m, INV_GETNUM => |s, player| {
            let slot = s.pop_int();
            let inv = pop_inv::<E>(s)?;
            let inventory = get_inv::<E>(inv, player)?;
            s.push_int(inventory.get(slot as u16).map(|x| x.num).unwrap_or(0) as i32);
        });

        // 4315
        active_player_mut!(m, INV_GETOBJ => |s, player| {
            let slot = s.pop_int();
            let inv = pop_inv::<E>(s)?;
            let inventory = get_inv::<E>(inv, player)?;
            s.push_int(inventory.get(slot as u16).map(|x| x.obj).map_or(-1, |v| v as i32));
        });

        // 4316
        active_player_mut!(m, INV_ITEMSPACE => |s, player| {
            let size = s.pop_int();
            let count = s.pop_int();
            let obj = pop_obj::<E>(s)?;
            let inv = pop_inv::<E>(s)?;
            if count == 0 {
                s.push_int(1); return Ok(());
            }
            if size < 0 || size > inv.size as i32 {
                return Err(ScriptError::Runtime(format!("size is out of range: {}", size)));
            }
            let remaining = inv_itemspace(inv, obj, get_inv::<E>(inv, player)?, count, size);
            s.push_int(if remaining == 0 { 1 } else { 0 });
        });

        // 4317
        active_player_mut!(m, INV_ITEMSPACE2 => |s, player| {
            let size = s.pop_int();
            let count = s.pop_int();
            let obj = pop_obj::<E>(s)?;
            let inv = pop_inv::<E>(s)?;
            if count == 0 {
                s.push_int(0); return Ok(());
            }
            s.push_int(inv_itemspace(inv, obj, get_inv::<E>(inv, player)?, count, size));
        });

        // 4318
        // https://x.com/JagexAsh/status/1706983568805704126
        active_player_mut!(m, INV_MOVEFROMSLOT => |s, player| {
            let slot = s.pop_int();
            let to_inv = pop_inv::<E>(s)?;
            let from_inv = pop_inv::<E>(s)?;
            require_inv_access(s, from_inv, s.int_operand() as usize)?;
            require_inv_access(s, to_inv, s.int_operand() as usize)?;
            let coord = player.coord();
            if from_inv.id == to_inv.id {
                let inv = get_inv_mut::<E>(from_inv.id, player)?;
                let Some(item) = inv.get(slot as u16).copied() else {
                    return Ok(());
                };
                let stackable = engine::<E>()
                    .objs()
                    .get_by_id(item.obj)
                    .is_some_and(|o| o.stackable);
                let overflow = inv.move_from_slot(slot as u16, stackable);
                if overflow > 0 {
                    let receiver37 = Some(player.uid().username37());
                    add_obj_split::<E>(coord, item.obj, overflow, stackable, receiver37, LOOTDROP_DURATION)?;
                }
            } else {
                let (from, to) = get_inv_pair_mut::<E>(from_inv.id, to_inv.id, player)?;
                let Some(item) = from.get(slot as u16).copied() else {
                    return Ok(());
                };
                let stackable = engine::<E>()
                    .objs()
                    .get_by_id(item.obj)
                    .is_some_and(|o| o.stackable);
                let overflow = from.move_from_slot_to(to, slot as u16, stackable);
                if overflow > 0 {
                    let receiver37 = Some(player.uid().username37());
                    add_obj_split::<E>(coord, item.obj, overflow, stackable, receiver37, LOOTDROP_DURATION)?;
                }
            }
        });

        // 4319
        // https://x.com/JagexAsh/status/1681616480763367424
        active_player_mut!(m, INV_MOVEITEM_CERT => |s, player| {
            let count = pop_count(s)?;
            let obj = pop_obj::<E>(s)?;
            let to_inv = pop_inv::<E>(s)?;
            let from_inv = pop_inv::<E>(s)?;
            require_inv_access(s, from_inv, s.int_operand() as usize)?;
            require_inv_access(s, to_inv, s.int_operand() as usize)?;
            let completed = get_inv_mut::<E>(from_inv.id, player)?.delete(obj.id, count);
            if completed == 0 {
                return Ok(());
            }
            let cert_id = cert(obj);
            let overflow = get_inv_mut::<E>(to_inv.id, player)?.add(cert_id, completed, true);
            if overflow > 0 {
                engine_mut::<E>().add_obj(player.coord(), cert_id, overflow, Some(player.uid().username37()), LOOTDROP_DURATION)?;
            }
        });

        // 4320
        // https://x.com/JagexAsh/status/1681616480763367424
        active_player_mut!(m, INV_MOVEITEM_UNCERT => |s, player| {
            let count = pop_count(s)?;
            let obj = pop_obj::<E>(s)?;
            let to_inv = pop_inv::<E>(s)?;
            let from_inv = pop_inv::<E>(s)?;
            require_inv_access(s, from_inv, s.int_operand() as usize)?;
            require_inv_access(s, to_inv, s.int_operand() as usize)?;
            let completed = get_inv_mut::<E>(from_inv.id, player)?.delete(obj.id, count);
            if completed == 0 {
                return Ok(());
            }
            let uncert_id = uncert(obj);
            let stackable = engine::<E>()
                .objs()
                .get_by_id(uncert_id)
                .is_some_and(|o| o.stackable);
            get_inv_mut::<E>(to_inv.id, player)?.add(uncert_id, completed, stackable);
        });

        // 4321
        // https://x.com/TheCrazy0neTv/status/1681181722811957248
        active_player_mut!(m, INV_MOVEITEM => |s, player| {
            let count = pop_count(s)?;
            let obj = pop_obj::<E>(s)?;
            let to_inv = pop_inv::<E>(s)?;
            let from_inv = pop_inv::<E>(s)?;
            require_inv_access(s, from_inv, s.int_operand() as usize)?;
            require_inv_access(s, to_inv, s.int_operand() as usize)?;
            let coord = player.coord();
            let obj_id = obj.id;
            let completed = get_inv_mut::<E>(from_inv.id, player)?.delete(obj_id, count);
            if completed == 0 {
                return Ok(());
            }
            let overflow = get_inv_mut::<E>(to_inv.id, player)?.add(obj_id, completed, obj.stackable);
            if overflow > 0 {
                let receiver37 = Some(player.uid().username37());
                add_obj_split::<E>(coord, obj_id, overflow, obj.stackable, receiver37, LOOTDROP_DURATION)?;
            }
        });

        // 4322
        // https://x.com/JagexAsh/status/1706983568805704126
        active_player_mut!(m, INV_MOVETOSLOT => |s, player| {
            let to_slot = s.pop_int();
            let from_slot = s.pop_int();
            let to_inv = pop_inv::<E>(s)?;
            let from_inv = pop_inv::<E>(s)?;
            require_inv_access(s, from_inv, s.int_operand() as usize)?;
            require_inv_access(s, to_inv, s.int_operand() as usize)?;
            if from_inv.id == to_inv.id {
                get_inv_mut::<E>(from_inv.id, player)?.move_to_slot(from_slot as u16, to_slot as u16);
            } else {
                let (from, to) = get_inv_pair_mut::<E>(from_inv.id, to_inv.id, player)?;
                from.move_to_slot_to(to, from_slot as u16, to_slot as u16);
            }
        });

        // 4323
        active_player_mut!(m, INV_SETSLOT => |s, player| {
            let count = pop_count(s)?;
            let obj = pop_obj::<E>(s)?;
            let slot = s.pop_int();
            let inv = pop_inv::<E>(s)?;
            require_inv_access(s, inv, s.int_operand() as usize)?;
            if !inv.dummyinv && obj.dummyitem != DummyItem::None {
                return Err(ScriptError::Runtime(format!("dummyitem in non-dummyinv: {:?} -> {:?}", obj.debugname(), inv.debugname())));
            }
            get_inv_mut::<E>(inv.id, player)?.set(slot as u16, obj.id, count);
        });

        // 4324
        none!(m, INV_SIZE => |s| {
            let inv = pop_inv::<E>(s)?;
            s.push_int(inv.size as i32);
        });

        // 4325
        none!(m, INV_STOCKBASE => |s| {
            let obj = pop_obj::<E>(s)?;
            let inv = pop_inv::<E>(s)?;
            let result = match (&inv.stockobj, &inv.stockcount) {
                (Some(stockobj), Some(stockcount)) => {
                    stockobj.iter().position(|&id| id == obj.id)
                        .map(|i| stockcount[i] as i32)
                        .unwrap_or(-1)
                }
                _ => -1,
            };
            s.push_int(result);
        });

        // 4326
        active_player_mut!(m, INV_STOPTRANSMIT => |s, player| {
            let com = s.pop_int() as u16;
            if player.has_inv_transmit(com).is_some() {
                player.clear_inv_transmits(com);
            }
        });

        // 4327
        active_player_mut!(m, INV_TOTAL => |s, player| {
            let obj = s.pop_int();
            let inv = pop_inv::<E>(s)?;
            if obj == -1 {
                s.push_int(0);
                return Ok(());
            }
            let inventory = get_inv::<E>(inv, player)?;
            s.push_int(inventory.total(obj as u16) as i32);
        });

        // 4328
        active_player_mut!(m, INV_TOTALCAT => |s, player| {
            let category = s.pop_int();
            let inv = pop_inv::<E>(s)?;
            let inv = get_inv::<E>(inv, player)?;
            let objs = engine::<E>().objs();
            let total: u32 = inv
                .slots
                .iter()
                .filter_map(|s| s.as_ref())
                .filter(|item| {
                    objs
                        .get_by_id(item.obj)
                        .and_then(|o| o.category)
                        .is_some_and(|cat| cat as i32 == category)
                })
                .map(|item| item.num)
                .fold(0u32, |a, b| a.saturating_add(b));
            s.push_int(total as i32);
        });

        // 4329
        active_player_mut!(m, INV_TOTALPARAM_STACK => |s, player| {
            let param = pop_param::<E>(s)?;
            let inv = pop_inv::<E>(s)?;
            s.push_int(inv_total_param::<E>(inv, param, true, player)?);
        });

        // 4330
        active_player_mut!(m, INV_TOTALPARAM => |s, player| {
            let param = pop_param::<E>(s)?;
            let inv = pop_inv::<E>(s)?;
            s.push_int(inv_total_param::<E>(inv, param, false, player)?);
        });

        // 4331
        active_player_mut!(m, INV_TRANSMIT => |s, player| {
            let com = s.pop_int() as u16;
            let inv = pop_inv::<E>(s)?;
            if player.has_inv_transmit(com).is_some_and(|id| id == inv.id) {
                return Ok(());
            }
            player.clear_inv_transmits(com);
            let stackmode = if inv.stackall { StackMode::Always } else { StackMode::Normal };
            match inv.scope {
                InvScope::Temp | InvScope::Perm => {
                    player.get_or_create_inv(inv.id, inv.size as usize, stackmode);
                }
                InvScope::Shared => {
                    engine_mut::<E>().get_shared_inv(inv.id, inv.size as usize, stackmode);
                }
            }
            player.add_inv_transmit(inv.id, com);
        });

        // 4332
        active_player_mut!(m, INVOTHER_TRANSMIT => |s, player| {
            // invother_transmit(player_uid, inv, component): component is on top.
            let com = s.pop_int();
            let inv = pop_inv::<E>(s)?;
            let uid = s.pop_int();
            if uid == -1 {
                return Err(ScriptError::Runtime("invother_transmit: uid is null".into()));
            }
            if com == -1 {
                return Err(ScriptError::Runtime("invother_transmit: com is null".into()));
            }
            let com = com as u16;
            if player.has_inv_other_transmit(com) == Some((uid, inv.id)) {
                return Ok(());
            }
            player.clear_inv_transmits(com);
            player.add_inv_other_transmit(com, inv.id, uid);
        });
    }
}
