use crate::engine::ScriptVars;
use crate::register::OpsRegistry;
use crate::session::{HostKind, PlayerVars, ScriptHost};
use crate::state::ScriptState;
use crate::{Result, ScriptError, active_vars, handlers};
#[cfg(since_254)]
use rs_pack::cache::provider::CacheType;
use rs_pack::cache::varp::VarPlayerType;
#[cfg(since_254)]
use rs_pack::cache::varpbit::VarpBitType;
use rs_pack::cache::{ScriptVarType, VarValue};
#[cfg(since_254)]
use rs_util::bits::{fits_bit_range, getbit_range, setbit_range_toint};
use rs_ver::TypeId;
use runec::opcodes::*;
use std::borrow::Cow;

/// Registers player variable, varpbit and client variable opcodes.
pub fn build<K: HostKind>() -> OpsRegistry<K>
where
    for<'a> K::Host<'a>: ScriptHost,
{
    handlers! { |m: OpsRegistry<K>|
        // 1
        active_vars!(m, PUSH_VARP => |s, id, vars| {
            let varp = vars.varp(id)?;
            let value = varp_value(vars.player, id, varp.var_type);
            if let VarValue::String(text) = value.as_ref() {
                s.push_string(text);
            } else {
                s.push_int(value.as_int());
            }
        });

        // 2
        active_vars!(m, POP_VARP => |s, id, vars| {
            let varp = vars.varp(id)?;
            if varp.protect && !protected(s) {
                return Err(ScriptError::Runtime(format!(
                    "Varp: {} requires protected access!", varp.debugname
                )));
            }
            let value = if varp.var_type == ScriptVarType::String {
                VarValue::String(s.pop_string())
            } else {
                VarValue::from_int(varp.var_type, s.pop_int())
            };
            vars.player.set_var(id, value, varp.transmit);
        });

        // 25
        #[cfg(since_254)]
        active_vars!(m, PUSH_VARPBIT => |s, id, vars| {
            let bit = vars.bit(id)?;
            let varp = vars.varp(bit.basevar)?;
            let value = varp_value(vars.player, bit.basevar, varp.var_type).as_int();
            s.push_int(getbit_range(value, bit.start_bit.into(), bit.end_bit.into()));
        });

        // 27
        #[cfg(since_254)]
        active_vars!(m, POP_VARPBIT => |s, id, vars| {
            let bit = vars.bit(id)?;
            let varp = vars.varp(bit.basevar)?;
            if varp.protect && !protected(s) {
                return Err(ScriptError::Runtime(format!(
                    "VarpBit: {} requires protected access!", bit.debugname()
                )));
            }
            vars.set_varbit(id, s.pop_int())?;
        });

        // 42
        #[cfg(since_530)]
        active_vars!(m, PUSH_VARC_INT => |s, id, vars| {
            s.push_int(vars.player.varc(id));
        });

        // 43
        #[cfg(since_530)]
        active_vars!(m, POP_VARC_INT => |s, id, vars| {
            vars.player.sendvarc(id, s.pop_int());
        });

        // 47
        #[cfg(since_530)]
        active_vars!(m, PUSH_VARC_STRING => |s, id, vars| {
            s.push_string(vars.player.varc_str(id));
        });

        // 48
        #[cfg(since_530)]
        active_vars!(m, POP_VARC_STRING => |s, id, vars| {
            vars.player.sendvarc_str(id, s.pop_string_ref());
        });
    }
}

impl<'a, V: ScriptVars> PlayerVars<'a, V> {
    fn varp(&self, id: u16) -> Result<&'a VarPlayerType> {
        self.varps
            .get_by_id(TypeId::new(id))
            .ok_or(ScriptError::VarpNotFound(id.into()))
    }

    #[cfg(since_254)]
    fn bit(&self, id: u16) -> Result<&'a VarpBitType> {
        self.bits
            .get_by_id(TypeId::new(id))
            .ok_or(ScriptError::VarpBitNotFound(id.into()))
    }

    /// Sets int varp `id`, sending it to the client when it transmits.
    pub fn set_varp(&mut self, id: u16, value: i32) -> Result<()> {
        let varp = self.varp(id)?;
        self.player
            .set_var(id, VarValue::from_int(varp.var_type, value), varp.transmit);
        Ok(())
    }

    /// Sets varpbit `id`, or clears it when `value` doesn't fit its bits,
    /// sending its varp to the client when that transmits.
    #[cfg(since_254)]
    pub fn set_varbit(&mut self, id: u16, value: i32) -> Result<()> {
        let bit = self.bit(id)?;
        let varp = self.varp(bit.basevar)?;
        let start = bit.start_bit.into();
        let end = bit.end_bit.into();
        let current = varp_value(self.player, bit.basevar, varp.var_type).as_int();
        let value = if fits_bit_range(value, start, end) {
            value
        } else {
            0
        };
        let value = VarValue::from_int(
            varp.var_type,
            setbit_range_toint(current, value, start, end),
        );
        self.player.set_var(bit.basevar, value, varp.transmit);
        Ok(())
    }
}

fn protected(state: &ScriptState) -> bool {
    let secondary = (state.int_operand() >> 16) != 0;
    state
        .pointers
        .has(ScriptState::PROTECTED_ACTIVE_PLAYER[usize::from(secondary)])
}

fn varp_value(player: &impl ScriptVars, id: u16, ty: ScriptVarType) -> Cow<'_, VarValue> {
    match player.stored_varp(id) {
        Some(value) => Cow::Borrowed(value),
        None => Cow::Owned(VarValue::default_for(ty)),
    }
}
