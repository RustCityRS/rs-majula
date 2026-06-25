use crate::engine::{ScriptEngine, ScriptNpc, ScriptPlayer, cache, engine, engine_mut};
use crate::iterators::{self, PlayerIteratorState};
use crate::register::OpsRegistry;
use crate::state::{ExecutionState, QueuePriority, ScriptArgument, ScriptState, TimerPriority};
use crate::trigger::ServerTriggerType;
use crate::util::*;
use crate::*;
use rs_grid::CoordGrid;
use rs_pack::cache::script::*;
use rs_pack::types::HuntCheckVis;
use rs_util::colour::rgb24_to_15;

/// Registers player-related opcodes covering stats, UI interfaces, movement,
/// combat, queues, timers, and player lookups.
///
/// # Opcodes Registered
///
/// Key opcodes include:
/// - **Identity / state:** `UID`, `COORD`, `DISPLAYNAME`, `GENDER`, `BUSY`, `BUSY2`,
///   `LOWMEM`, `PLAYERMEMBER`, `STAFFMODLEVEL`, `RUNENERGY`
/// - **Stats:** `STAT`, `STAT_BASE`, `STAT_ADD`, `STAT_SUB`, `STAT_DRAIN`,
///   `STAT_HEAL`, `STAT_BOOST`, `STAT_ADVANCE`, `STAT_RANDOM`
/// - **Animations:** `ANIM`, `READYANIM`, `RUNANIM`, `TURNANIM`, `WALKANIM`,
///   `WALKANIM_B`, `WALKANIM_L`, `WALKANIM_R`, `SPOTANIM_PL`
/// - **Interfaces:** `IF_CLOSE`, `IF_OPENCHAT`, `IF_OPENMAIN`, `IF_OPENMAIN_SIDE`,
///   `IF_SETANIM`, `IF_SETCOLOUR`, `IF_SETHIDE`, `IF_SETNPCHEAD`, `IF_SETOBJECT`,
///   `IF_SETPLAYERHEAD`, `IF_SETPOSITION`, `IF_SETRESUMEBUTTONS`, `IF_SETTAB`, `IF_SETTEXT`
/// - **Movement:** `FACESQUARE`, `P_WALK`, `P_TELEJUMP`, `P_TELEPORT`, `P_EXACTMOVE`,
///   `P_RUN`, `P_ARRIVEDELAY`, `WALKTRIGGER`, `GETWALKTRIGGER`
/// - **Combat / hero:** `DAMAGE`, `BOTH_HEROPOINTS`, `FINDHERO`, `HEADICONS_GET`,
///   `HEADICONS_SET`, `P_ANIMPROTECT`
/// - **Queues / timers:** `QUEUE`, `QUEUEVARARG`, `SETTIMER`, `CLEARTIMER`, `GETTIMER`
/// - **Delayed execution:** `P_DELAY`, `P_COUNTDIALOG`, `P_PAUSEBUTTON`
/// - **Player search:** `FINDUID`, `P_FINDUID`, `HUNTALL`, `HUNTNEXT`
/// - **Interactions:** `P_OPLOC`, `P_OPNPC`, `P_OPOBJ`, `P_STOPACTION`,
///   `P_CLEARPENDINGACTION`, `P_LOCMERGE`
/// - **Audio:** `MES`, `MIDI_JINGLE`, `MIDI_SONG`, `SOUND_SYNTH`
/// - **Misc:** `AFK_EVENT`, `BUILDAPPEARANCE`, `CAM_RESET`, `LAST_COM`, `LAST_INT`,
///   `LAST_ITEM`, `LAST_SLOT`, `LAST_TARGETSLOT`, `LAST_USEITEM`, `LAST_USESLOT`,
///   `P_LOGOUT`, `P_PREVENTLOGOUT`, `P_APRANGE`, `SESSION_LOG`, `WEALTH_EVENT`
///
/// # Call Stack
///
/// **Called by:** `Engine::new` (in `rs-engine/src/engine.rs`) via `ops::player::build`
/// **Calls:** `OpsRegistry::new`, `OpsRegistry::insert` via the `handlers!` / `active_player!` /
/// `active_player_mut!` / `protected_active_player_mut!` / `none!` macros
pub fn build<E: ScriptEngine + 'static>() -> OpsRegistry {
    handlers! { |m|
        // 2000
        active_player_mut!(m, AFK_EVENT => |s, player| {
            s.push_int(player.afk_event() as i32);
        });

        // 2001
        active_player_mut!(m, ALLOWDESIGN => |s, player| {
            let allow = s.pop_int();
            player.set_allow_design(allow == 1);
        });

        // 2002
        active_player_mut!(m, ANIM => |s, player| {
            let delay = s.pop_int_as::<u8>()?;
            let seq = s.pop_int();
            player.anim((seq != -1).then_some(seq as u16), delay);
        });

        // 2003
        // https://x.com/JagexAsh/status/1799020087086903511
        none!(m, BOTH_HEROPOINTS => |s| {
            let damage = s.pop_int();
            let secondary = s.int_operand() == 1;
            let from_uid = if secondary { s.active_player2 } else { s.active_player };
            let to_uid = if secondary { s.active_player } else { s.active_player2 };
            let from = from_uid.ok_or(ScriptError::Runtime("from player is null".into()))?;
            let to = to_uid.ok_or(ScriptError::Runtime("to player is null".into()))?;
            let user37 = from.username37();
            engine_mut::<E>()
                .get_player_mut(to.pid())
                .ok_or(ScriptError::Runtime("to player not found".into()))?
                .heropoints(user37, damage);
        });

        // 2004
        active_player_mut!(m, BUILDAPPEARANCE => |s, player| {
            let inv = pop_inv(s)?;
            player.buildappearance(inv.id);
        });

        // 2005
        // https://x.com/JagexAsh/status/1653407769989349377
        active_player!(m, BUSY => |s, player| {
            let busy = player.busy() || player.logging_out();
            s.push_int(busy as i32);
        });

        // 2006
        // https://x.com/JagexAsh/status/1791053667228856563
        active_player!(m, BUSY2 => |s, player| {
            let busy2 = player.has_interaction() || player.has_waypoints();
            s.push_int(busy2 as i32);
        });

        // 2007
        active_player_mut!(m, CAM_LOOKAT => |s, player| {
            let rate2 = s.pop_int_as::<u8>()?;
            let rate = s.pop_int_as::<u8>()?;
            let height = s.pop_int_as::<u16>()?;
            let coord = CoordGrid::from(s.pop_int() as u32);
            player.cam_lookat(coord.x(), coord.z(), height, rate, rate2)?;
        });

        // 2008
        active_player_mut!(m, CAM_MOVETO => |s, player| {
            let rate2 = s.pop_int_as::<u8>()?;
            let rate = s.pop_int_as::<u8>()?;
            let height = s.pop_int_as::<u16>()?;
            let coord = CoordGrid::from(s.pop_int() as u32);
            player.cam_moveto(coord.x(), coord.z(), height, rate, rate2)?;
        });

        // 2009
        active_player_mut!(m, CAM_RESET => |_s, player| {
            player.cam_reset();
        });

        // 2010
        active_player_mut!(m, CAM_SHAKE => |s, player| {
            let frequency = s.pop_int_as::<u8>()?;
            let amplitude = s.pop_int_as::<u8>()?;
            let jitter = s.pop_int_as::<u8>()?;
            let direction = s.pop_int_as::<u8>()?;
            player.cam_shake(direction, jitter, amplitude, frequency);
        });

        // 2011
        // https://x.com/JagexAsh/status/1821831590906859683
        active_player_mut!(m, CLEARQUEUE => |s, player| {
            let script_id = s.pop_int();
            player.clearqueue(script_id);
        });

        // 2012
        active_player_mut!(m, CLEARSOFTTIMER => |s, player| {
            let script = pop_script::<E>(s)?;
            player.cleartimer(script.id);
        });

        // 2013
        active_player_mut!(m, CLEARTIMER => |s, player| {
            let script = pop_script::<E>(s)?;
            player.cleartimer(script.id);
        });

        // 2014
        active_player!(m, COORD => |s, player| {
            s.push_int(player.coord() as i32);
        });

        // 2015
        none!(m, DAMAGE => |s| {
            let amount = s.pop_int_as::<u8>()?;
            let damage_type = s.pop_int_as::<u8>()?;
            let uid = s.pop_int();
            let pid = (uid & 0x7FF) as u16;
            if let Some(target) = engine_mut::<E>().get_player_mut(pid)
                && target.uid().packed() as i32 == uid
            {
                target.damage(amount, damage_type);
            }
        });

        // 2016
        active_player!(m, DISPLAYNAME => |s, player| {
            s.push_string(&player.uid().screen_name());
        });

        // 2017
        active_player_mut!(m, FACESQUARE => |s, player| {
            let coord = CoordGrid::from(s.pop_int() as u32);
            player.facesquare(coord.x(), coord.z());
        });

        // 2018
        // https://x.com/JagexAsh/status/1799020087086903511
        active_player!(m, FINDHERO => |s, player| {
            let Some(user37) = player.findhero() else {
                s.push_int(0);
                return Ok(());
            };
            let Some(uid) = engine::<E>().find_player_by_user37(user37) else {
                s.push_int(0);
                return Ok(());
            };
            set_active_player(s, uid, true);
            s.push_int(1);
        });

        // 2019
        none!(m, FINDUID => |s| {
            let uid = s.pop_int();
            let operand = s.int_operand();
            let pid = (uid & 0x7FF) as u16;
            let player = engine::<E>().get_player(pid);
            match player {
                Some(player) if player.uid().packed() as i32 == uid => {
                    set_active_player(s, player.uid(), operand != 0);
                    s.push_int(1);
                }
                _ => s.push_int(0),
            }
        });

        // 2020
        active_player!(m, GENDER => |s, player| {
            s.push_int(player.gender() as i32);
        });

        // 2021
        // https://x.com/JagexAsh/status/1821831590906859683
        active_player!(m, GETQUEUE => |s, player| {
            let script_id = s.pop_int();
            s.push_int(player.getqueue(script_id));
        });

        // 2022
        active_player!(m, GETTIMER => |s, player| {
            let script_id = pop_script::<E>(s)?.id;
            s.push_int(player.gettimer(script_id));
        });

        // 2023
        // https://x.com/JagexAsh/status/1779778790593372205
        active_player!(m, GETWALKTRIGGER => |s, player| {
            s.push_int(player.getwalktrigger());
        });

        // 2024
        active_player!(m, HEADICONS_GET => |s, player| {
            s.push_int(player.headicons_get() as i32);
        });

        // 2025
        active_player_mut!(m, HEADICONS_SET => |s, player| {
            player.headicons_set(s.pop_int_as::<u8>()?);
        });

        // 2026
        active_player_mut!(m, HEALENERGY => |s, player| {
            // 100=1%, 1000=10%, 10000=100%
            let amount = s.pop_int();
            player.healenergy(amount);
        });

        // 2027
        active_player_mut!(m, HINT_COORD => |s, player| {
            let height = s.pop_int_as::<u8>()?;
            let coord = CoordGrid::from(s.pop_int() as u32);
            let offset = s.pop_int_as::<u8>()?;
            player.hint_tile(offset, coord.x(), coord.z(), height);
        });

        // 2028
        active_player_mut!(m, HINT_NPC => |s, player| {
            let secondary = s.int_operand() != 0;
            let nid = if secondary { s.active_npc2 } else { s.active_npc }
                .ok_or_else(|| ScriptError::Runtime("no active_npc".into()))?
                .nid();
            player.hint_npc(nid);
        });

        // 2029
        active_player_mut!(m, HINT_PL => |s, player| {
            // `active_player2` is the player opposite the operand-selected active player.
            let secondary = s.int_operand() != 0;
            let slot = if secondary { s.active_player } else { s.active_player2 }
                .ok_or_else(|| ScriptError::Runtime("no active_player2".into()))?
                .pid();
            player.hint_player(slot);
        });

        // 2030
        active_player_mut!(m, HINT_STOP => |_s, player| {
            player.stop_hint();
        });

        // 2031
        none!(m, HUNTALL => |s| {
            let vis = HuntCheckVis::try_from(s.pop_int() as u8).unwrap_or(HuntCheckVis::Off);
            let distance = s.pop_int();
            let coord = CoordGrid::from(s.pop_int() as u32);

            let players = iterators::hunt_players::<E>(coord, distance, vis);
            s.player_iterator = Some(PlayerIteratorState {
                matches: players,
                cursor: 0,
            });
        });

        // 2032
        none!(m, HUNTNEXT => |s| {
            let iter = match s.player_iterator.as_mut() {
                Some(iter) => iter,
                None => {
                    s.push_int(0);
                    return Ok(());
                }
            };
            if iter.cursor < iter.matches.len() {
                let pid = iter.matches[iter.cursor];
                iter.cursor += 1;
                if let Some(player) = engine::<E>().get_player(pid) {
                    let operand = s.int_operand();
                    set_active_player(s, player.uid(), operand != 0);
                    s.push_int(1);
                } else {
                    s.push_int(0);
                }
            } else {
                s.push_int(0);
            }
        });

        // 2033
        active_player_mut!(m, IF_CLOSE => |_s, player| {
            player.if_close(true)?;
        });

        // 2034
        active_player_mut!(m, IF_OPENCHAT => |s, player| {
            player.if_openchat(s.pop_int_as::<u16>()?);
        });

        // 2035
        active_player_mut!(m, IF_OPENMAIN_SIDE => |s, player| {
            let side = s.pop_int_as::<u16>()?;
            let com = s.pop_int_as::<u16>()?;
            player.if_openmain_side(com, side);
        });

        // 2036
        active_player_mut!(m, IF_OPENMAIN => |s, player| {
            player.if_openmain(s.pop_int_as::<u16>()?);
        });

        // 2037
        active_player_mut!(m, IF_OPENSIDE => |s, player| {
            player.if_openside(s.pop_int_as::<u16>()?);
        });

        // 2038
        active_player_mut!(m, IF_SETANIM => |s, player| {
            let seq = s.pop_int();
            let com = s.pop_int_as::<u16>()?;
            if seq == -1 {
                return Ok(());
            }
            let seq = cache()
                .seqs
                .get_by_id(seq as u16)
                .ok_or(ScriptError::SeqNotFound(seq))?;
            player.if_setanim(com, seq.id);
        });

        // 2039
        active_player_mut!(m, IF_SETCOLOUR => |s, player| {
            let colour = s.pop_int();
            let com = s.pop_int_as::<u16>()?;
            player.if_setcolour(com, rgb24_to_15(colour));
        });

        // 2040
        active_player_mut!(m, IF_SETHIDE => |s, player| {
            let hide = s.pop_int();
            let com = s.pop_int_as::<u16>()?;
            player.if_sethide(com, hide == 1);
        });

        // 2041
        active_player_mut!(m, IF_SETMODEL => |s, player| {
            let model = s.pop_int_as::<u16>()?;
            let com = s.pop_int_as::<u16>()?;
            player.if_setmodel(com, model);
        });

        // 2042
        active_player_mut!(m, IF_SETNPCHEAD => |s, player| {
            let npc = s.pop_int_as::<u16>()?;
            let com = s.pop_int_as::<u16>()?;
            player.if_setnpchead(com, npc);
        });

        // 2043
        active_player_mut!(m, IF_SETOBJECT => |s, player| {
            let scale = s.pop_int_as::<u16>()?;
            let obj = s.pop_int_as::<u16>()?;
            let com = s.pop_int_as::<u16>()?;
            player.if_setobject(com, obj, scale);
        });

        // 2044
        active_player_mut!(m, IF_SETPLAYERHEAD => |s, player| {
            player.if_setplayerhead(s.pop_int_as::<u16>()?);
        });

        // 2045
        active_player_mut!(m, IF_SETPOSITION => |s, player| {
            let y = s.pop_int_as::<u16>()?;
            let x = s.pop_int_as::<u16>()?;
            let com = s.pop_int_as::<u16>()?;
            player.if_setposition(com, x, y);
        });

        // 2046
        active_player_mut!(m, IF_SETRECOL => |s, player| {
            let dst = s.pop_int_as::<u16>()?;
            let src = s.pop_int_as::<u16>()?;
            let com = s.pop_int_as::<u16>()?;
            player.if_setrecol(com, src, dst);
        });

        // 2047
        active_player_mut!(m, IF_SETRESUMEBUTTONS => |s, player| {
            let button5 = s.pop_int();
            let button4 = s.pop_int();
            let button3 = s.pop_int();
            let button2 = s.pop_int();
            let button1 = s.pop_int();
            player.if_setresumebuttons(Some(vec![button1, button2, button3, button4, button5]));
        });

        // 2048
        active_player_mut!(m, IF_SETTAB => |s, player| {
            let tab = s.pop_int_as::<u8>()?;
            // com can be null for example: tutorial island purposes.
            let com = s.pop_int();
            player.if_settab(com as u16, tab);
        });

        // 2049
        active_player_mut!(m, IF_SETTABACTIVE => |s, player| {
            player.if_settabactive(s.pop_int_as::<u8>()?);
        });

        // 2050
        active_player_mut!(m, IF_SETTEXT => |s, player| {
            let text = s.pop_string();
            let com = s.pop_int_as::<u16>()?;
            player.if_settext(com, &text);
        });

        // 2051
        active_player!(m, LAST_COM => |s, player| {
            s.push_int(player.last_com());
        });

        // 2052
        // https://x.com/JagexAsh/status/1782377050021523947
        none!(m, LAST_INT => |s| {
            s.push_int(s.last_int.unwrap_or(-1));
        });

        // 2053
        active_player!(m, LAST_ITEM => |s, player| {
            if let Some(trigger) = s.trigger && !trigger.allows_last_item() {
                return Err(ScriptError::Runtime(
                    format!("{} is not safe to use in this trigger: {:?}", LAST_ITEM, s.trigger)
                ))
            }
            s.push_int(player.last_item());
        });

        // 2054
        active_player_mut!(m, LAST_LOGIN_INFO => |_s, player| {
            player.last_login_info();
        });

        // 2055
        active_player!(m, LAST_SLOT => |s, player| {
            if let Some(trigger) = s.trigger && !trigger.allows_last_slot() {
                return Err(ScriptError::Runtime(
                    format!("{} is not safe to use in this trigger: {:?}", LAST_SLOT, s.trigger)
                ))
            }
            s.push_int(player.last_slot());
        });

        // 2056
        active_player!(m, LAST_TARGETSLOT => |s, player| {
            if let Some(trigger) = s.trigger && !trigger.allows_last_targetslot() {
                return Err(ScriptError::Runtime(
                    format!("{} is not safe to use in this trigger: {:?}", LAST_TARGETSLOT, s.trigger)
                ))
            }
            s.push_int(player.last_targetslot());
        });

        // 2057
        active_player!(m, LAST_USEITEM => |s, player| {
            if let Some(trigger) = s.trigger && !trigger.allows_last_useitem() {
                return Err(ScriptError::Runtime(
                    format!("{} is not safe to use in this trigger: {:?}", LAST_USEITEM, s.trigger)
                ))
            }
            s.push_int(player.last_useitem());
        });

        // 2058
        active_player!(m, LAST_USESLOT => |s, player| {
            if let Some(trigger) = s.trigger && !trigger.allows_last_useslot() {
                return Err(ScriptError::Runtime(
                    format!("{} is not safe to use in this trigger: {:?}", LAST_USESLOT, s.trigger)
                ))
            }
            s.push_int(player.last_useslot());
        });

        // 2059
        active_player_mut!(m, LONGQUEUE => |s, player| {
            let logout_action = s.pop_int();
            let arg = s.pop_int();
            let delay = s.pop_int();
            let script = pop_script::<E>(s)?;
            player.queue(
                script.id,
                QueuePriority::Long,
                delay as u16,
                Some(vec![ScriptArgument::Int(logout_action), ScriptArgument::Int(arg)]),
            )?;
        });

        // 2060
        active_player_mut!(m, LONGQUEUEVARARG => |s, player| {
            let args = s.pop_script_args();
            let logout_action = s.pop_int();
            let delay = s.pop_int();
            let script = pop_script::<E>(s)?;
            let mut queue_args = Vec::with_capacity(args.len() + 1);
            queue_args.push(ScriptArgument::Int(logout_action));
            queue_args.extend(args);
            player.queue(script.id, QueuePriority::Long, delay as u16, Some(queue_args))?;
        });

        // 2061
        active_player!(m, LOWMEM => |s, player| {
            s.push_int(player.lowmem() as i32);
        });

        // 2062
        active_player_mut!(m, MES => |s, player| {
            let text = s.pop_string();
            player.mes(&text);
        });

        // 2063
        active_player_mut!(m, MIDI_JINGLE => |s, player| {
            #[cfg(rev = "225")]
            {
                s.pop_int();
                let jingle = pop_jingle(s)?;
                if player.lowmem() {
                    return Ok(());
                }
                player.midi_jingle(jingle.length_ms as u16, &jingle.data);
            }
            #[cfg(since_244)]
            {
                let delay = s.pop_int();
                let name = s.pop_string();
                if player.lowmem() {
                    return Ok(());
                }
                if let Some(id) = jingle_midi_id(&name) {
                    player.midi_jingle(id, delay as u16);
                }
            }
        });

        // 2064
        active_player_mut!(m, MIDI_SONG => |s, player| {
            #[cfg(rev = "225")]
            {
                let song = pop_song(s)?;
                if player.lowmem() {
                    return Ok(());
                }
                player.midi_song(&song.name, song.crc, song.data.len() as i32);
            }
            #[cfg(since_244)]
            {
                let name = s.pop_string();
                if player.lowmem() {
                    return Ok(());
                }
                if let Some(id) = song_midi_id(&name) {
                    player.midi_song(id);
                }
            }
        });

        // 2065
        active_player!(m, NAME => |s, player| {
            s.push_string(&player.uid().username());
        });

        // 2066
        active_player_mut!(m, P_ANIMPROTECT => |s, player| {
            player.animprotect(s.pop_int() != 0);
        });

        // 2067
        protected_active_player_mut!(m, P_APRANGE => |s, player| {
            let range = s.pop_int();
            player.aprange(range);
        });

        // 2068
        // https://x.com/JagexAsh/status/1648254846686904321
        protected_active_player_mut!(m, P_ARRIVEDELAY => |s, player| {
            if player.arrivedelay(engine::<E>().clock()) {
                s.execution = ExecutionState::Suspended;
            }
        });

        // 2069
        // https://x.com/JagexAsh/status/1780230057023181259
        active_player_mut!(m, P_CLEARPENDINGACTION => |_s, player| {
            player.clearpendingaction()?;
        });

        // 2070
        active_player_mut!(m, P_COUNTDIALOG => |s, player| {
            player.countdialog();
            s.execution = ExecutionState::CountDialog;
        });

        // 2071
        // https://x.com/JagexAsh/status/1684478874703343616
        // https://x.com/JagexAsh/status/1780932943038345562
        active_player_mut!(m, P_DELAY => |s, player| {
            let delay = s.pop_int();
            let clock = engine::<E>().clock();
            player.delay(clock + 1 + delay as u32);
            s.execution = ExecutionState::Suspended;
        });

        // 2072
        // https://x.com/JagexAsh/status/1684174294086033410
        protected_active_player_mut!(m, P_EXACTMOVE => |s, player| {
            let direction = s.pop_int();
            let finish = s.pop_int();
            let begin = s.pop_int();
            let end = CoordGrid::from(s.pop_int() as u32);
            let start = CoordGrid::from(s.pop_int() as u32);
            player.clear_waypoints();
            player.exactmove(
                start.x(),
                start.z(),
                end.x(),
                end.z(),
                begin as u16,
                finish as u16,
                direction as u8,
            );
        });

        // 2073
        none!(m, P_FINDUID => |s| {
            let uid = s.pop_int();
            let operand = s.int_operand();
            let pid = (uid & 0x7FF) as u16;
            if s.pointers.has(ScriptState::PROTECTED_ACTIVE_PLAYER[operand as usize]) {
                let active = match operand { 0 => s.active_player, _ => s.active_player2 };
                if let Some(active) = active && active.packed() as i32 == uid {
                    s.push_int(1); return Ok(());
                }
            }
            let player = engine::<E>().get_player(pid);
            match player {
                Some(player) => {
                    if !player.can_access() {
                        s.push_int(0); return Ok(());
                    }
                    set_active_player(s, player.uid(), operand != 0);
                    s.pointers.add(ScriptState::PROTECTED_ACTIVE_PLAYER[operand as usize]);
                    s.push_int(1);
                }
                None => s.push_int(0)
            }
        });

        // 2074
        // https://x.com/JagexAsh/status/1684232225397657602
        protected_active_player_mut!(m, P_LOCMERGE => |s, player| {
            let nw = CoordGrid::from(s.pop_int() as u32);
            let se = CoordGrid::from(s.pop_int() as u32);
            let end = s.pop_int_as::<u16>()?;
            let start = s.pop_int_as::<u16>()?;

            let loc = get_active_loc(s, s.int_operand() != 0)?;
            let pid = player.uid().pid();

            engine_mut::<E>().merge_loc(
                loc.coord,
                loc.shape,
                loc.angle,
                loc.id,
                start,
                end,
                pid,
                se.z(),
                se.x(),
                nw.z(),
                nw.x(),
            );
        });

        // 2075
        protected_active_player_mut!(m, P_LOGOUT => |_s, player| {
            player.logout(true);
        });

        // 2076
        active_player!(m, P_OPHELD => |s, _player| {
            Err(ScriptError::Runtime("Unimplemented.".to_string()))?;
        });

        // 2077
        // https://x.com/JagexAsh/status/1791472651623370843
        protected_active_player_mut!(m, P_OPLOC => |s, player| {
            let op = s.pop_int() - 1;
            if !(0..5).contains(&op) {
                return Err(ScriptError::Runtime(format!("Invalid oploc: {}", op + 1)));
            }

            let loc = s.active_loc
                .ok_or_else(|| ScriptError::Runtime("no active_loc".into()))?;

            let loc_type = cache()
                .locs
                .get_by_id(loc.id)
                .ok_or(ScriptError::LocNotFound(loc.id as i32))?;

            if let Some(ops) = &loc_type.op {
                if ops.get(op as usize).is_none_or(|o| o.is_none()) {
                    return Ok(());
                }
            } else {
                return Ok(());
            }

            player.stopaction()?;

            let forceapproach = loc_type.forceapproach as u8;
            if !player.in_operable_distance_loc(
                loc.coord, loc_type.width, loc_type.length, loc.shape, loc.angle, forceapproach,
            ) {
                let coord = CoordGrid::from(loc.coord);
                player.queue_waypoint(coord.x(), coord.z());
            }

            let trigger = ServerTriggerType::ApLoc1 as u8 + op as u8;
            player.set_interaction_loc(
                loc.coord, loc.id, loc_type.width, loc_type.length,
                loc.shape, loc.angle, loc.layer, trigger,
            );
        });

        // 2078
        // https://x.com/JagexAsh/status/1791472651623370843
        protected_active_player_mut!(m, P_OPNPC => |s, player| {
            let op = s.pop_int() - 1;
            if !(0..5).contains(&op) {
                return Err(ScriptError::Runtime(format!("Invalid opnpc: {}", op + 1)));
            }

            let secondary = s.int_operand() != 0;
            let npc_uid = if secondary { s.active_npc2 } else { s.active_npc }
                .ok_or_else(|| ScriptError::Runtime("no active_npc".into()))?;

            let npc = engine::<E>()
                .get_npc(npc_uid.nid())
                .ok_or_else(|| ScriptError::Runtime(format!("active npc slot empty: {}", npc_uid.nid())))?;

            let id = npc.uid().id();

            let npc_type = cache()
                .npcs
                .get_by_id(id)
                .ok_or(ScriptError::NpcNotFound(id as i32))?;

            if let Some(ops) = &npc_type.op {
                if ops.get(op as usize).is_none_or(|o| o.is_none()) {
                    return Ok(());
                }
            } else {
                return Ok(());
            }

            player.stopaction()?;

            let trigger = ServerTriggerType::ApNpc1 as u8 + op as u8;
            player.set_interaction_npc(npc_uid.nid(), trigger);
        });

        // 2079
        protected_active_player_mut!(m, P_OPNPCT => |s, player| {
            let spell = s.pop_int();
            if spell == -1 {
                return Err(ScriptError::Runtime("opnpct: spell is null".into()));
            }

            let secondary = s.int_operand() != 0;
            let nid = if secondary { s.active_npc2 } else { s.active_npc }
                .ok_or_else(|| ScriptError::Runtime("no active_npc".into()))?
                .nid();

            player.stopaction()?;
            player.set_interaction_npc(nid, ServerTriggerType::ApNpcT as u8);
            player.set_interaction_spell(spell as u16);
        });

        // 2080
        // https://x.com/JagexAsh/status/1791472651623370843
        // https://x.com/JagexAsh/status/1790684996480442796
        protected_active_player_mut!(m, P_OPOBJ => |s, player| {
            let op = s.pop_int() - 1;
            if !(0..5).contains(&op) {
                return Err(ScriptError::Runtime(format!("Invalid opobj: {}", op + 1)));
            }

            let obj = s.active_obj
                .ok_or_else(|| ScriptError::Runtime("no active_obj".into()))?;

            let obj_type = cache()
                .objs
                .get_by_id(obj.id)
                .ok_or(ScriptError::ObjNotFound(obj.id as i32))?;

            if let Some(ops) = &obj_type.op {
                if ops.get(op as usize).is_none_or(|o| o.is_none()) {
                    return Ok(());
                }
            } else {
                return Ok(());
            }

            player.stopaction()?;

            let coord = CoordGrid::from(obj.coord);
            player.queue_waypoint(coord.x(), coord.z());

            let trigger = ServerTriggerType::ApObj1 as u8 + op as u8;
            player.set_interaction_obj(obj.coord, obj.id, obj.count, trigger);
        });

        // 2081
        protected_active_player_mut!(m, P_OPPLAYER => |s, player| {
            let op = s.pop_int() - 1;
            if !(0..5).contains(&op) {
                return Err(ScriptError::Runtime(format!("Invalid opplayer: {}", op + 1)));
            }

            let Some(target) = s.active_player2 else {
                return Ok(());
            };

            player.stopaction()?;
            player.set_interaction_player(
                target.pid(),
                ServerTriggerType::ApPlayer1 as u8 + op as u8,
            );
        });

        // 2082
        protected_active_player_mut!(m, P_OPPLAYERT => |s, player| {
            let spell = s.pop_int();
            if spell == -1 {
                return Err(ScriptError::Runtime("opplayert: spell is null".into()));
            }

            let Some(target) = s.active_player2 else {
                return Ok(());
            };

            player.stopaction()?;
            player.set_interaction_player(target.pid(), ServerTriggerType::ApPlayerT as u8);
            player.set_interaction_spell(spell as u16);
        });

        // 2083
        // https://x.com/JagexAsh/status/1389465615631519744
        protected_active_player_mut!(m, P_PAUSEBUTTON => |s, _player| {
            s.execution = ExecutionState::PauseButton;
        });

        // 2084
        protected_active_player_mut!(m, P_PREVENTLOGOUT => |s, player| {
            let message = s.pop_string();
            let duration = s.pop_int();
            let clock = engine::<E>().clock();
            player.prevent_logout(&message, clock + duration as u32);
        });

        // 2085
        protected_active_player_mut!(m, P_RUN => |s, player| {
            player.run(s.pop_int_as::<u8>()?);
        });

        // 2086
        // https://x.com/JagexAsh/status/1780904271610867780
        protected_active_player_mut!(m, P_STOPACTION => |_s, player| {
            player.stopaction()?;
        });

        // 2087
        // https://x.com/JagexAsh/status/1697517518007541917
        protected_active_player_mut!(m, P_TELEJUMP => |s, player| {
            let coord = s.pop_int() as u32;
            player.telejump(coord);
        });

        // 2088
        // https://x.com/JagexAsh/status/1697517518007541917
        // https://x.com/JagexAsh/status/1790684996480442796
        protected_active_player_mut!(m, P_TELEPORT => |s, player| {
            let coord = s.pop_int() as u32;
            player.teleport(coord);
        });

        // 2089
        // https://x.com/JagexAsh/status/1605130887292751873
        // https://x.com/JagexAsh/status/1698248664349614138
        protected_active_player_mut!(m, P_WALK => |s, player| {
            let coord = CoordGrid::from(s.pop_int() as u32);
            player.walk(coord.x(), coord.z());
        });

        // 2090
        active_player!(m, PLAYERMEMBER => |s, player| {
            s.push_int(player.member() as i32);
        });

        // 2091
        active_player!(m, PROJANIM_PL => |s, player| {
            let arc = s.pop_int_as::<u8>()?;
            let peak = s.pop_int_as::<u8>()?;
            let duration = s.pop_int_as::<u16>()?;
            let delay = s.pop_int_as::<u16>()?;
            let dst_height = s.pop_int_as::<u8>()?;
            let src_height = s.pop_int_as::<u8>()?;
            let spotanim = pop_spotanim(s)?;
            let uid = s.pop_int();
            let src = CoordGrid::from(s.pop_int() as u32);
            let dst = CoordGrid::from(player.coord());
            if uid != player.uid().packed() as i32 {
                return Err(ScriptError::Runtime(format!("Invalid uid: {}, expected: {}", uid, player.uid().packed() as i32)))
            }
            // Players target as -pid - 1 (NPCs target as index + 1).
            let target = -(player.uid().pid() as i16) - 1;
            engine_mut::<E>().map_proj_anim(
                src.y(),
                src.x(),
                src.z(),
                dst.x(),
                dst.z(),
                target,
                spotanim.id,
                src_height << 2,
                dst_height << 2,
                delay,
                duration,
                peak,
                arc
            );
        });

        // 2092
        active_player_mut!(m, QUEUE => |s, player| {
            let arg = s.pop_int();
            let delay = s.pop_int_as::<u16>()?;
            let script = pop_script::<E>(s)?;
            player.queue(script.id, QueuePriority::Normal, delay, Some(vec![ScriptArgument::Int(arg)]))?;
        });

        // 2093
        active_player_mut!(m, QUEUEVARARG => |s, player| {
            let args = s.pop_script_args();
            let delay = s.pop_int_as::<u16>()?;
            let script = pop_script::<E>(s)?;
            player.queue(script.id, QueuePriority::Normal, delay, Some(args))?;
        });

        // 2094
        active_player_mut!(m, READYANIM => |s, player| {
            // anim can be null for example: agility
            player.readyanim(s.pop_int() as u16);
        });

        // 2095
        active_player_mut!(m, RUNANIM => |s, player| {
            // anim can be null for example: agility
            player.runanim(s.pop_int() as u16);
        });

        // 2096
        active_player!(m, RUNENERGY => |s, player| {
            s.push_int(player.runenergy() as i32);
        });

        // 2097
        active_player_mut!(m, SAY => |s, player| {
            let msg = s.pop_string();
            player.say(&msg);
        });

        // 2098
        active_player_mut!(m, SESSION_LOG => |s, _player| {
            s.pop_int();
            s.pop_string();
            // TODO
        });

        // 2099
        active_player_mut!(m, SETGENDER => |s, player| {
            player.setgender(s.pop_int_as::<u8>()?);
        });

        // 2100
        active_player_mut!(m, SETIDKIT => |s, player| {
            let colour = s.pop_int_as::<u8>()?;
            let idk = pop_idk(s)?;
            let body_type = idk.body_type as u8;
            let idk_id = idk.id;
            player.setidkit(body_type, idk_id, colour);
        });

        // 2101
        active_player_mut!(m, SETSKINCOLOUR => |s, player| {
            player.setskincolour(s.pop_int_as::<u8>()?);
        });

        // 2102
        active_player_mut!(m, SETTIMER => |s, player| {
            let args = s.pop_script_args();
            let interval = s.pop_int_as::<u16>()?;
            let script = pop_script::<E>(s)?;
            let clock = engine::<E>().clock();
            player.settimer(script.id, TimerPriority::Normal, interval, clock, Some(args));
        });

        // 2103
        active_player_mut!(m, SOFTTIMER => |s, player| {
            let args = s.pop_script_args();
            let interval = s.pop_int_as::<u16>()?;
            let script = pop_script::<E>(s)?;
            let clock = engine::<E>().clock();
            player.settimer(script.id, TimerPriority::Soft, interval, clock, Some(args));
        });

        // 2104
        active_player_mut!(m, SOUND_SYNTH => |s, player| {
            let delay = s.pop_int_as::<u16>()?;
            let loops = s.pop_int_as::<u8>()?;
            let synth = s.pop_int();
            if player.lowmem() || synth == -1 {
                return Ok(());
            }
            player.sound_synth(synth as u16, loops, delay);
        });

        // 2105
        active_player_mut!(m, SPOTANIM_PL => |s, player| {
            let delay = s.pop_int_as::<u16>()?;
            let height = s.pop_int_as::<u16>()?;
            let spotanim = pop_spotanim(s)?;
            player.spotanim(spotanim.id, height, delay);
        });

        // 2106
        active_player!(m, STAFFMODLEVEL => |s, player| {
            s.push_int(player.staffmodlevel() as i32);
        });

        // 2107
        active_player_mut!(m, STAT_ADD => |s, player| {
            let percent = s.pop_int();
            let constant = s.pop_int();
            let stat = s.pop_int() as usize;
            player.stat_add(stat, constant, percent);
        });

        // 2108
        active_player_mut!(m, STAT_ADVANCE => |s, player| {
            let xp = s.pop_int();
            let stat = s.pop_int() as usize;
            player.add_xp(stat, xp.saturating_mul(engine::<E>().multi_experience() as i32));
        });

        // 2109
        active_player!(m, STAT_BASE => |s, player| {
            let stat = s.pop_int() as usize;
            s.push_int(player.stat_base(stat) as i32);
        });

        // 2110
        active_player_mut!(m, STAT_BOOST => |s, player| {
            let percent = s.pop_int();
            let constant = s.pop_int();
            let stat = s.pop_int() as usize;
            player.stat_boost(stat, constant, percent);
        });

        // 2111
        active_player_mut!(m, STAT_DRAIN => |s, player| {
            let percent = s.pop_int();
            let constant = s.pop_int();
            let stat = s.pop_int() as usize;
            player.stat_drain(stat, constant, percent);
        });

        // 2112
        active_player_mut!(m, STAT_HEAL => |s, player| {
            let percent = s.pop_int();
            let constant = s.pop_int();
            let stat = s.pop_int() as usize;
            player.stat_heal(stat, constant, percent);
        });

        // 2113
        // https://x.com/JagexAsh/status/1110604592138670083
        active_player!(m, STAT_RANDOM => |s, player| {
            let high = s.pop_int();
            let low = s.pop_int();
            let stat = s.pop_int() as usize;
            let level = player.stat(stat) as i32;
            let value = (low * (99 - level)) / 98 + (high * (level - 1)) / 98 + 1;
            let chance = (engine_mut::<E>().random().next_double() * 256.0) as i32;
            s.push_int(if value > chance { 1 } else { 0 });
        });

        // 2114
        active_player_mut!(m, STAT_SUB => |s, player| {
            let percent = s.pop_int();
            let constant = s.pop_int();
            let stat = s.pop_int() as usize;
            player.stat_sub(stat, constant, percent);
        });

        // 2115
        active_player!(m, STAT_TOTAL => |s, player| {
            s.push_int(player.stat_total());
        });

        // 2116
        active_player!(m, STAT => |s, player| {
            let stat = s.pop_int() as usize;
            s.push_int(player.stat(stat) as i32);
        });

        // 2117
        // https://x.com/JagexAsh/status/1698973910048403797
        active_player_mut!(m, STRONGQUEUE => |s, player| {
            let arg = s.pop_int();
            let delay = s.pop_int_as::<u16>()?;
            let script = pop_script::<E>(s)?;
            player.queue(script.id, QueuePriority::Strong, delay, Some(vec![ScriptArgument::Int(arg)]))?;
        });

        // 2118
        // https://x.com/JagexAsh/status/1698973910048403797
        active_player_mut!(m, STRONGQUEUEVARARG => |s, player| {
            let args = s.pop_script_args();
            let delay = s.pop_int_as::<u16>()?;
            let script = pop_script::<E>(s)?;
            player.queue(script.id, QueuePriority::Strong, delay, Some(args))?;
        });

        // 2119
        active_player_mut!(m, TURNANIM => |s, player| {
            // anim can be null for example: agility
            player.turnanim(s.pop_int() as u16);
        });

        // 2120
        active_player_mut!(m, TUT_CLOSE => |_s, player| {
            player.tut_close()?;
        });

        // 2121
        active_player_mut!(m, TUT_FLASH => |s, player| {
            player.tut_flash(s.pop_int_as::<u8>()?);
        });

        // 2122
        active_player_mut!(m, TUT_OPEN => |s, player| {
            player.tut_open(s.pop_int_as::<u16>()?);
        });

        // 2123
        active_player!(m, UID => |s, player| {
            s.push_int(player.uid().packed() as i32);
        });

        // 2124
        active_player_mut!(m, WALKANIM_B => |s, player| {
            // anim can be null for example: agility
            player.walkanim_b(s.pop_int() as u16);
        });

        // 2125
        active_player_mut!(m, WALKANIM_L => |s, player| {
            // anim can be null for example: agility
            player.walkanim_l(s.pop_int() as u16);
        });

        // 2126
        active_player_mut!(m, WALKANIM_R => |s, player| {
            // anim can be null for example: agility
            player.walkanim_r(s.pop_int() as u16);
        });

        // 2127
        active_player_mut!(m, WALKANIM => |s, player| {
            // anim can be null for example: agility
            player.walkanim(s.pop_int() as u16);
        });

        // 2128
        active_player_mut!(m, WALKTRIGGER => |s, player| {
            let trigger = s.pop_int();
            player.walktrigger(trigger);
        });

        // 2129
        // https://x.com/JagexAsh/status/1698973910048403797
        active_player_mut!(m, WEAKQUEUE => |s, player| {
            let arg = s.pop_int();
            let delay = s.pop_int_as::<u16>()?;
            let script = pop_script::<E>(s)?;
            player.queue(script.id, QueuePriority::Weak, delay, Some(vec![ScriptArgument::Int(arg)]))?;
        });

        // 2130
        // https://x.com/JagexAsh/status/1698973910048403797
        active_player_mut!(m, WEAKQUEUEVARARG => |s, player| {
            let args = s.pop_script_args();
            let delay = s.pop_int_as::<u16>()?;
            let script = pop_script::<E>(s)?;
            player.queue(script.id, QueuePriority::Weak, delay, Some(args))?;
        });

        // 2131
        active_player_mut!(m, WEALTH_EVENT => |s, _player| {
            let _name = s.pop_string();
            let _value = s.pop_int();
            let _count = s.pop_int();
            let _event = s.pop_int();
            // TODO
        });

        // 2132
        protected_active_player!(m, WEIGHT => |s, player| {
            s.push_int(player.weight());
        });

        // 2133
        active_player_mut!(m, SETIDKCOLOUR => |s, player| {
            let colour = s.pop_int_as::<u8>()?;
            let slot = s.pop_int_as::<u8>()?;
            player.setidkcolour(slot, colour)?;
        });

        // 2134
        // https://x.com/JagexAsh/status/1694990340669747261
        active_player!(m, BUFFER_FULL => |s, player| {
            s.push_int(player.buffer_full() as i32);
        });

        // 2135
        // TODO: this is duplicated with `LOWMEM`
        active_player!(m, LOWMEMORY => |s, player| {
            s.push_int(player.lowmem() as i32);
        });

        // 2136
        // TODO: this is duplicated with `HINT_PL`
        active_player_mut!(m, HINT_PLAYER => |s, player| {
            // `active_player2` is the player opposite the operand-selected active player.
            let secondary = s.int_operand() != 0;
            let slot = if secondary { s.active_player } else { s.active_player2 }
                .ok_or_else(|| ScriptError::Runtime("no active_player2".into()))?
                .pid();
            player.hint_player(slot);
        });

        // 2137
        // TODO: this is duplicated with `RUNANIM`
        active_player_mut!(m, BAS_RUNNING => |s, player| {
            // anim can be null for example: agility
            player.runanim(s.pop_int() as u16);
        });

        // 2138
        // TODO: this is duplicated with `READYANIM`
        active_player_mut!(m, BAS_READYANIM => |s, player| {
            // anim can be null for example: agility
            player.readyanim(s.pop_int() as u16);
        });

        // 2139
        // TODO: this is duplicated with `WALKANIM`
        active_player_mut!(m, BAS_WALK_F => |s, player| {
            // anim can be null for example: agility
            player.walkanim(s.pop_int() as u16);
        });

        // 2140
        // TODO: this is duplicated with `WALKANIM_B`
        active_player_mut!(m, BAS_WALK_B => |s, player| {
            // anim can be null for example: agility
            player.walkanim_b(s.pop_int() as u16);
        });

        // 2141
        // TODO: this is duplicated with `WALKANIM_L`
        active_player_mut!(m, BAS_WALK_L => |s, player| {
            // anim can be null for example: agility
            player.walkanim_l(s.pop_int() as u16);
        });

        // 2142
        // TODO: this is duplicated with `WALKANIM_R`
        active_player_mut!(m, BAS_WALK_R => |s, player| {
            // anim can be null for example: agility
            player.walkanim_r(s.pop_int() as u16);
        });

        // 2143
        // TODO: this is duplicated with `TURNANIM`
        active_player_mut!(m, BAS_TURNONSPOT => |s, player| {
            // anim can be null for example: agility
            player.turnanim(s.pop_int() as u16);
        });
    }
}
