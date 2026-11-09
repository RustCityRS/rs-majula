use crate::engine::Engine;
use crate::region::REGION_X;
use rs_entity::Controller;
use rs_grid::CoordGrid;
use rs_pack::cache::VarValue;
use rs_pack::types::ParamValue;
use rs_var::VarSet;
use rs_vm::state::{QueuePriority, ScriptArgument};
use rs_vm::trigger::ServerTriggerType;
use rs_vm::{ControllerUid, ScriptError};
use rustc_hash::FxHashMap;

/// The world's controllers, by slot and by coord.
#[derive(Default)]
pub struct Controllers {
    slots: Vec<Option<Controller>>,
    generations: Vec<u16>,
    free: Vec<u16>,
    at: FxHashMap<u32, Vec<u16>>,
}

impl Controllers {
    pub fn add(
        &mut self,
        coord: CoordGrid,
        id: u16,
        lifecycle_at: Option<u32>,
        vars: VarSet,
    ) -> Option<ControllerUid> {
        let slot = match self.free.pop() {
            Some(slot) => slot,
            None => {
                let slot = u16::try_from(self.slots.len()).ok()?;
                self.slots.push(None);
                self.generations.push(0);
                slot
            }
        };
        let generation = self.generations[slot as usize].wrapping_add(1);
        self.generations[slot as usize] = generation;
        let uid = ControllerUid::new(slot, generation);
        self.slots[slot as usize] = Some(Controller::new(uid, id, coord, lifecycle_at, vars));
        self.at.entry(coord.packed()).or_default().push(slot);
        Some(uid)
    }

    pub fn get(&self, uid: ControllerUid) -> Option<&Controller> {
        self.slots
            .get(uid.slot() as usize)
            .and_then(Option::as_ref)
            .filter(|controller| controller.uid == uid)
    }

    pub fn get_mut(&mut self, uid: ControllerUid) -> Option<&mut Controller> {
        self.slots
            .get_mut(uid.slot() as usize)
            .and_then(Option::as_mut)
            .filter(|controller| controller.uid == uid)
    }

    /// The first controller added at `coord` of those still there.
    pub fn first_at(&self, coord: CoordGrid) -> Option<&Controller> {
        self.at
            .get(&coord.packed())?
            .first()
            .and_then(|&slot| self.slots[slot as usize].as_ref())
    }

    pub fn find_exact(&self, coord: CoordGrid, id: u16) -> Option<ControllerUid> {
        self.at.get(&coord.packed())?.iter().find_map(|&slot| {
            self.slots[slot as usize]
                .as_ref()
                .filter(|controller| controller.id == id)
                .map(|controller| controller.uid)
        })
    }

    pub fn remove(&mut self, uid: ControllerUid) {
        if self.get(uid).is_none() {
            return;
        }
        let slot = uid.slot();
        let controller = self.slots[slot as usize].take().unwrap();
        let key = controller.coord.packed();
        if let Some(slots) = self.at.get_mut(&key) {
            slots.retain(|&other| other != slot);
            if slots.is_empty() {
                self.at.remove(&key);
            }
        }
        self.free.push(slot);
    }

    /// Removes every controller standing where `inside` holds.
    pub fn remove_where(&mut self, inside: impl Fn(CoordGrid) -> bool) {
        let uids: Vec<ControllerUid> = self
            .slots
            .iter()
            .flatten()
            .filter(|controller| inside(controller.coord))
            .map(|controller| controller.uid)
            .collect();
        for uid in uids {
            self.remove(uid);
        }
    }
}

impl Engine {
    pub(crate) fn controller_add(
        &mut self,
        coord: CoordGrid,
        id: u16,
        duration: i32,
    ) -> Result<ControllerUid, ScriptError> {
        let placed = if coord.x() < REGION_X {
            rsmod::is_zone_allocated(coord.x(), coord.z(), coord.y())
        } else {
            self.regions.find(coord.x(), coord.z()).is_some()
        };
        if !placed {
            return Err(ScriptError::Runtime(format!(
                "controller_add: no zone or region at {coord:?}"
            )));
        }
        let lifecycle_at = (duration > 0).then_some(duration as u32);
        let mut vars = VarSet::new(self.cache.varcons.types.iter().map(|v| v.var_type));
        if let Some(controller) = self.cache.controllers.get_by_id(id) {
            for (varcon, value) in controller.vars.iter() {
                let Some(varcon) = self.cache.varcons.get_by_id(*varcon) else {
                    continue;
                };
                vars.set(
                    varcon.id,
                    match value {
                        ParamValue::Int(value) => VarValue::from_int(varcon.var_type, *value),
                        ParamValue::String(value) => VarValue::String(value.to_string()),
                    },
                );
            }
        }
        self.controllers
            .add(coord, id, lifecycle_at, vars)
            .ok_or_else(|| ScriptError::Runtime("controller_add: no free controller".into()))
    }

    pub(crate) fn controller(&self, uid: ControllerUid) -> Result<&Controller, ScriptError> {
        self.controllers
            .get(uid)
            .ok_or_else(|| ScriptError::Runtime("controller no longer exists".into()))
    }

    fn controller_mut(&mut self, uid: ControllerUid) -> Result<&mut Controller, ScriptError> {
        self.controllers
            .get_mut(uid)
            .ok_or_else(|| ScriptError::Runtime("controller no longer exists".into()))
    }

    pub(crate) fn controller_queue(
        &mut self,
        uid: ControllerUid,
        arg: i32,
        delay: u16,
        source: Option<u16>,
    ) -> Result<(), ScriptError> {
        self.controller_mut(uid)?.queues.add_from(
            QueuePriority::Normal,
            ServerTriggerType::AiConQueue as i32,
            delay,
            Some(vec![ScriptArgument::Int(arg)]),
            source,
        )
    }

    pub(crate) fn controller_queue_script(
        &mut self,
        uid: ControllerUid,
        script: i32,
        args: Vec<ScriptArgument>,
        delay: u16,
        source: Option<u16>,
    ) -> Result<(), ScriptError> {
        self.controller_mut(uid)?
            .queues
            .add_named(script, delay, args, source);
        Ok(())
    }

    pub(crate) fn controller_settimer(
        &mut self,
        uid: ControllerUid,
        interval: i32,
    ) -> Result<(), ScriptError> {
        let controller = self.controller_mut(uid)?;
        controller.timer_interval = u16::try_from(interval).ok().filter(|&i| i > 0);
        controller.timer_clock = 0;
        Ok(())
    }

    pub(crate) fn controller_var(
        &self,
        uid: ControllerUid,
        id: u16,
    ) -> Result<VarValue, ScriptError> {
        let controller = self.controller(uid)?;
        if id as usize >= controller.vars.len() {
            return Err(ScriptError::Runtime(format!("Varcon not found: {id}")));
        }
        Ok(controller.vars.get(id).clone())
    }

    pub(crate) fn set_controller_var(
        &mut self,
        uid: ControllerUid,
        id: u16,
        value: VarValue,
    ) -> Result<(), ScriptError> {
        let controller = self.controller_mut(uid)?;
        if id as usize >= controller.vars.len() {
            return Err(ScriptError::Runtime(format!("Varcon not found: {id}")));
        }
        controller.vars.set(id, value);
        Ok(())
    }

    /// Runs each controller in slot order as the npc phase runs npcs: its
    /// despawn countdown, then its timer, then its queue. A controller
    /// deleted by a script runs nothing more. Its own phase, right after the
    /// npc phase.
    pub(crate) fn process_controllers(&mut self) {
        for slot in 0..self.controllers.slots.len() {
            let Some(controller) = self.controllers.slots[slot].as_mut() else {
                continue;
            };
            let uid = controller.uid;
            if let Some(remaining) = controller.lifecycle_at {
                if remaining > 1 {
                    controller.lifecycle_at = Some(remaining - 1);
                } else {
                    self.controllers.remove(uid);
                    continue;
                }
            }
            self.controller_process_timer(uid);
            self.controller_process_queue(uid);
        }
    }

    fn controller_process_timer(&mut self, uid: ControllerUid) {
        let Some(controller) = self.controllers.get_mut(uid) else {
            return;
        };
        let Some(interval) = controller.timer_interval else {
            return;
        };
        controller.timer_clock += 1;
        if controller.timer_clock < interval {
            return;
        }
        controller.timer_clock = 0;
        let id = controller.id as i32;
        let Some(script) = self.script_by_key(ServerTriggerType::AiConTimer, Some(id), None) else {
            return;
        };
        let mut state = self.build_state(script, None, None, None);
        state.active_controller = Some(uid);
        self.run_world_script(state);
    }

    fn controller_process_queue(&mut self, uid: ControllerUid) {
        let Some(controller) = self.controllers.get_mut(uid) else {
            return;
        };
        let id = controller.id;
        let mut h = controller.queues.queue.head();
        while let Some(idx) = h {
            let Some(controller) = self.controllers.get_mut(uid) else {
                return;
            };
            let queue = &mut controller.queues.queue;
            queue[idx].delay = queue[idx].delay.saturating_sub(1);
            if queue[idx].delay == 0 {
                let request = queue.unlink(idx);
                if let Some(script) = self.queued_script(&request, id, None) {
                    let last_int = request.last_int();
                    let mut state = self.build_state(script, None, None, request.args);
                    state.active_controller = Some(uid);
                    state.last_int = last_int;
                    state.queue_source = request.source;
                    self.run_world_script(state);
                }
            }
            h = self
                .controllers
                .get_mut(uid)
                .and_then(|controller| controller.queues.queue.next());
        }
    }
}
