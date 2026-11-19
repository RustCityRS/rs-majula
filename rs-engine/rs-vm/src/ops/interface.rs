#[cfg(since_530)]
use crate::Result;
use crate::register::OpsRegistry;
#[cfg(since_530)]
use crate::state::ScriptState;
use crate::util::{overlay_button, pop_com};
use crate::{ScriptError, active_interface, handlers};
#[cfg(since_530)]
use rs_pack::types::IfEvents;
use rs_util::colour::rgb24_to_15;
use rs_ver::TypeId;
use runec::opcodes::*;

/// Registers interface and clientscript opcodes.
pub fn build() -> OpsRegistry {
    handlers! { |m|
        // 2033
        active_interface!(m, IF_CLOSE => |_s, player| {
            player.if_close(true)?;
        });

        // 2034
        active_interface!(m, IF_OPENCHAT => |s, player| {
            let com = s.pop_int_as::<u16>()?;
            if overlay_button(s) {
                player.if_close_others(com)?;
            }
            player.if_openchat(com);
        });

        // 2035
        active_interface!(m, IF_OPENMAIN_SIDE => |s, player| {
            let side = s.pop_int_as::<u16>()?;
            let com = s.pop_int_as::<u16>()?;
            if overlay_button(s) {
                player.if_close_others(com)?;
            }
            player.if_openmain_side(com, side);
        });

        // 2036
        active_interface!(m, IF_OPENMAIN => |s, player| {
            let com = s.pop_int_as::<u16>()?;
            if overlay_button(s) {
                player.if_close_others(com)?;
            }
            player.if_openmain(com);
        });

        // 2037
        active_interface!(m, IF_OPENSIDE => |s, player| {
            let com = s.pop_int_as::<u16>()?;
            if overlay_button(s) {
                player.if_close_others(com)?;
            }
            player.if_openside(com);
        });

        // 2038
        active_interface!(m, IF_SETANIM => |s, player, seq_count| {
            let id = s.pop_int();
            let com = pop_com(s)?;
            if id != -1 {
                let seq = TypeId::try_from(id)
                    .ok()
                    .filter(|seq| seq.index() < seq_count)
                    .ok_or(ScriptError::SeqNotFound(id))?;
                player.if_setanim(com, seq);
            }
        });

        // 2039
        active_interface!(m, IF_SETCOLOUR => |s, player| {
            let colour = s.pop_int();
            let com = pop_com(s)?;
            player.if_setcolour(com, rgb24_to_15(colour));
        });

        // 2040
        active_interface!(m, IF_SETHIDE => |s, player| {
            let hide = s.pop_int();
            let com = pop_com(s)?;
            player.if_sethide(com, hide == 1);
        });

        // 2041
        active_interface!(m, IF_SETMODEL => |s, player| {
            let model = s.pop_int_as::<TypeId>()?;
            let com = pop_com(s)?;
            player.if_setmodel(com, model);
        });

        // 2042
        active_interface!(m, IF_SETNPCHEAD => |s, player| {
            let npc = s.pop_int_as::<TypeId>()?;
            let com = pop_com(s)?;
            player.if_setnpchead(com, npc);
        });

        // 2043
        active_interface!(m, IF_SETOBJECT => |s, player| {
            let scale = s.pop_int();
            let obj = s.pop_int_as::<TypeId>()?;
            let com = pop_com(s)?;
            player.if_setobject(com, obj, scale);
        });

        // 2044
        active_interface!(m, IF_SETPLAYERHEAD => |s, player| {
            let com = pop_com(s)?;
            player.if_setplayerhead(com);
        });

        // 2045
        active_interface!(m, IF_SETPOSITION => |s, player| {
            let y = s.pop_int();
            let x = s.pop_int();
            let com = pop_com(s)?;
            player.if_setposition(com, x as u16, y as u16);
        });

        // 2046
        #[cfg(before_244)]
        active_interface!(m, IF_SETRECOL => |s, player| {
            let dst = s.pop_int_as::<u16>()?;
            let src = s.pop_int_as::<u16>()?;
            let com = s.pop_int_as::<u16>()?;
            player.if_setrecol(com, src, dst);
        });

        // 2047
        #[cfg(any(before_254, since_530))]
        active_interface!(m, IF_SETRESUMEBUTTONS => |s, player| {
            let mut buttons = vec![0; 5];
            for button in buttons.iter_mut().rev() {
                *button = s.pop_int();
            }
            player.if_setresumebuttons(Some(buttons));
        });

        // 2048
        active_interface!(m, IF_SETTAB => |s, player| {
            let tab = s.pop_int_as::<u8>()?;
            let com = s.pop_int();
            player.if_settab(com as u16, tab);
        });

        // 2049
        active_interface!(m, IF_SETTABACTIVE => |s, player| {
            player.if_settabactive(s.pop_int_as::<u8>()?);
        });

        // 2050
        active_interface!(m, IF_SETTEXT => |s, player| {
            let com = pop_com(s)?;
            player.if_settext(com, s.pop_string_ref());
        });

        // 2144
        #[cfg(since_245_2)]
        active_interface!(m, IF_SETSCROLLPOS => |s, player| {
            let y = s.pop_int_as::<u16>()?;
            let com = pop_com(s)?;
            player.if_setscrollpos(com, y);
        });

        // 2145
        #[cfg(since_244)]
        active_interface!(m, IF_OPENOVERLAY => |s, player| {
            player.if_openoverlay(s.pop_int() as u16);
        });

        // 2149
        #[cfg(since_254)]
        active_interface!(m, IF_ADDRESUMEBUTTON => |s, player| {
            player.if_addresumebutton(s.pop_int());
        });

        // 2161
        #[cfg(since_530)]
        active_interface!(m, IF_CLOSESUB => |s, player| {
            let component_hash = s.pop_int();
            player.if_closesub(component_hash);
        });

        // 2162
        #[cfg(since_530)]
        active_interface!(m, IF_MOVESUB => |s, player| {
            let to = s.pop_int();
            let from = s.pop_int();
            player.if_movesub(from, to);
        });

        // 2164
        #[cfg(since_530)]
        active_interface!(m, IF_OPENSUB => |s, player| {
            let parent_hash = s.pop_int();
            let sub_id = s.pop_int_as::<u16>()?;
            player.if_opensub(sub_id, parent_hash);
        });

        // 2165
        #[cfg(since_530)]
        active_interface!(m, IF_OPENTOP => |s, player| {
            let com = s.pop_int_as::<u16>()?;
            player.if_opentop(com, 0);
        });

        // 2166
        #[cfg(since_530)]
        active_interface!(m, IF_OPENTOP_MODE => |s, player| {
            let mode = s.pop_int_as::<u8>()?;
            let com = s.pop_int_as::<u16>()?;
            player.if_opentop(com, mode);
        });

        // 2167
        #[cfg(since_530)]
        active_interface!(m, IF_SETEVENTS => |s, player| {
            let can_target = s.pop_int() != 0;
            let allow_drag = s.pop_int() != 0;
            let depth = s.pop_int();
            let useon_flags = s.pop_int();
            let ops = s.pop_int();
            let slot2 = event_slot(s)?;
            let slot1 = event_slot(s)?;
            let com = s.pop_int();

            if slot1 > slot2 {
                return Err(ScriptError::Runtime("invalid interface slot range".into()));
            }
            let settings = IfEvents::new(ops, useon_flags, depth, allow_drag, can_target);
            player.if_setevents(com, slot1, slot2, settings);
        });

        // 2168
        #[cfg(since_647)]
        active_interface!(m, IF_SETGRAPHIC => |s, player| {
            let graphic = s.pop_int_as::<TypeId>()?;
            let com = s.pop_int();
            player.if_setgraphic(com, graphic);
        });

        // 2186
        #[cfg(since_530)]
        active_interface!(m, RUNCLIENTSCRIPT => |s, player| {
            let script_id = s.pop_int();
            player.runclientscript(script_id);
        });

        // 2187
        #[cfg(since_530)]
        active_interface!(m, RUNCLIENTSCRIPTVARARG => |s, player| {
            let args = s.pop_script_args();
            let script_id = s.pop_int();
            player.runclientscript_with_params(script_id, args);
        });
    }
}

#[cfg(since_530)]
fn event_slot(state: &mut ScriptState) -> Result<i32> {
    match state.pop_int() {
        -1 | 65535 => Ok(-1),
        slot @ 0..=65534 => Ok(slot),
        _ => Err(ScriptError::Runtime("invalid interface slot range".into())),
    }
}
