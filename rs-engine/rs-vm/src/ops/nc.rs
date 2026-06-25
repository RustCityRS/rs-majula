use crate::register::OpsRegistry;
use crate::util::{pop_npc, pop_param};
use crate::{handlers, none};
use rs_pack::ParamValue;
use rs_pack::cache::script::*;

/// Registers NPC config (NC) opcodes for querying static NPC definitions
/// from the cache, such as name, category, size, and parameters.
///
/// # Opcodes Registered
///
/// Key opcodes include:
/// - **Identity:** `NC_NAME`, `NC_DEBUGNAME`, `NC_DESC`, `NC_CATEGORY`
/// - **Properties:** `NC_SIZE`, `NC_VISLEVEL`
/// - **Interactions:** `NC_OP`
/// - **Params:** `NC_PARAM`
///
/// # Call Stack
///
/// **Called by:** `Engine::new` (in `rs-engine/src/engine.rs`) via `ops::nc::build`
/// **Calls:** `OpsRegistry::new`, `OpsRegistry::insert` via the `handlers!` / `none!` macros
pub fn build() -> OpsRegistry {
    handlers! { |m|
        // 4000
        none!(m, NC_CATEGORY => |s| {
            let npc = pop_npc(s)?;
            s.push_int(npc.category.map(|x| x as i32).unwrap_or(-1));
        });

        // 4001
        none!(m, NC_DEBUGNAME => |s| {
            let npc = pop_npc(s)?;
            s.push_string(npc.debugname().unwrap_or("null"));
        });

        // 4002
        none!(m, NC_DESC => |s| {
            let npc = pop_npc(s)?;
            s.push_string(npc.desc.as_deref().unwrap_or("null"));
        });

        // 4003
        none!(m, NC_NAME => |s| {
            let npc = pop_npc(s)?;
            s.push_string(npc.name.as_deref().unwrap_or(npc.debugname().unwrap_or("null")));
        });

        // 4004
        none!(m, NC_OP => |s| {
            let op = s.pop_int();
            let npc = pop_npc(s)?;
            let name = npc.op
                .as_ref()
                .and_then(|ops| ops.get((op - 1) as usize))
                .and_then(|o| o.as_deref())
                .unwrap_or("");
            s.push_string(name);
        });

        // 4005
        none!(m, NC_PARAM => |s| {
            let param = pop_param(s)?;
            let obj = pop_npc(s)?;
            let value = obj.params
                .as_ref()
                .and_then(|p| param.get_param_or_default(p))
                .cloned()
                .unwrap_or_else(|| param.default_param());
            match value {
                ParamValue::Int(v) => s.push_int(v),
                ParamValue::String(v) => s.push_string(&v),
            }
        });

        // 4006
        none!(m, NC_SIZE => |s| {
            let npc = pop_npc(s)?;
            s.push_int(npc.size as i32);
        });

        // 4007
        none!(m, NC_VISLEVEL => |s| {
            let npc = pop_npc(s)?;
            s.push_int(npc.vislevel.map(|x| x as i32).unwrap_or(-1));
        });
    }
}
