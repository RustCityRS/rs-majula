use crate::player::LobbyPlayer;
use rs_pack::cache::script::Script;
use rs_pack::startup::LobbyAssets;
use rs_vm::ops::{core, debug, interface, vars};
use rs_vm::pointer::ScriptPointer;
use rs_vm::register::OpsRegistry;
use rs_vm::session::{PlayerVars, ScriptHost, ScriptInterface};
use rs_vm::state::ScriptState;
use rs_vm::trigger::ServerTriggerType;
use rs_vm::{Result, ScriptError, vm};
use std::sync::Arc;
use tracing::error;

#[cfg(test)]
mod tests;

pub(crate) struct LobbyVm {
    ops: OpsRegistry,
}

impl LobbyVm {
    pub fn new(assets: &LobbyAssets) -> Self {
        let mut available = core::language();
        available.extend(debug::build());
        available.extend(interface::build());
        available.extend(vars::build());

        let mut ops = OpsRegistry::new();
        for &id in assets.commands.id_to_debugname.keys() {
            if let Ok(opcode) = u16::try_from(id)
                && let Some(handler) = available.get(opcode)
            {
                ops.insert(opcode, handler);
            }
        }
        Self { ops }
    }
}

pub(crate) struct LobbySession<'a> {
    pub player: &'a mut LobbyPlayer,
    pub assets: &'a LobbyAssets,
}

impl ScriptHost for LobbySession<'_> {
    fn get_script(&self, id: i32) -> Option<&Arc<Script>> {
        self.assets.scripts.get_by_id(id)
    }

    fn seq_count(&self) -> usize {
        self.assets.seq_count
    }

    fn interface(
        &mut self,
        _state: &ScriptState,
        secondary: bool,
    ) -> Result<&mut dyn ScriptInterface> {
        if secondary {
            return Err(ScriptError::NoSubject);
        }
        Ok(self)
    }

    fn vars(&mut self, _state: &ScriptState, secondary: bool) -> Result<PlayerVars<'_>> {
        if secondary {
            return Err(ScriptError::NoSubject);
        }
        let assets = self.assets;
        Ok(PlayerVars {
            player: self,
            varps: &assets.varps,
            bits: &assets.varpbits,
        })
    }
}

impl LobbyPlayer {
    pub fn trigger(
        &mut self,
        assets: &LobbyAssets,
        vm: &LobbyVm,
        trigger: ServerTriggerType,
        component: Option<i32>,
    ) -> bool {
        self.run_trigger(assets, vm, trigger, component)
            .unwrap_or(true)
    }

    pub fn run_trigger(
        &mut self,
        assets: &LobbyAssets,
        vm: &LobbyVm,
        trigger: ServerTriggerType,
        component: Option<i32>,
    ) -> Result<bool> {
        let base = trigger as i32;
        let script = component
            .and_then(|com| assets.scripts.get_by_lookup((com << 10) | (2 << 8) | base))
            .or_else(|| assets.scripts.get_by_lookup(base));
        let Some(script) = script else {
            return Ok(false);
        };
        let mut state = ScriptState::init(Arc::clone(script), None, None, None);
        state.trigger = Some(trigger);
        state.pointers.add(ScriptPointer::ActivePlayer);
        state.pointers.add(ScriptPointer::ProtectedActivePlayer);
        if let Err(err) = vm::run(
            &mut state,
            &vm.ops,
            &mut LobbySession {
                player: self,
                assets,
            },
        ) {
            error!(
                "Lobby trigger {trigger:?} for {}: script {} error at pc={}: {err}",
                self.user37,
                state.script.info.name,
                state.pc()
            );
            return Err(err);
        }
        Ok(true)
    }
}
