use crate::engine::Engine;
use crate::region::REGION_X;
use rs_grid::CoordGrid;
use rs_pack::cache::VarValue;
use rs_pack::types::ParamValue;
use rs_var::VarSet;
use rs_vm::trigger::ServerTriggerType;
use rs_vm::{ControllerUid, ScriptError};
use rustc_hash::FxHashMap;
use std::mem;

/// A controller: an invisible entity that runs a type's `[ai_contimer]` and
/// `[ai_conqueue]` scripts at a coord, keeping its own vars.
pub struct Controller {
    pub uid: ControllerUid,
    pub id: u16,
    pub coord: CoordGrid,
    despawn: Option<u32>,
    timer: Option<(u32, u32)>,
    queue: Vec<(u32, i32)>,
    pub vars: VarSet,
}

/// The world's controllers, by slot and by coord.
#[derive(Default)]
pub struct Controllers {
    slots: Vec<Option<Controller>>,
    generations: Vec<u16>,
    free: Vec<u16>,
    at: FxHashMap<u32, Vec<u16>>,
}

/// A controller script due this tick.
enum Due {
    Timer,
    Queue(i32),
}

impl Controllers {
    pub fn add(
        &mut self,
        coord: CoordGrid,
        id: u16,
        despawn: Option<u32>,
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
        self.slots[slot as usize] = Some(Controller {
            uid,
            id,
            coord,
            despawn,
            timer: None,
            queue: Vec::new(),
            vars,
        });
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

    /// Takes what each controller has due at `clock`: its despawn, its
    /// timer, then its queue in the order queued.
    fn due(
        &mut self,
        clock: u32,
        out: &mut Vec<(ControllerUid, Due)>,
        gone: &mut Vec<ControllerUid>,
    ) {
        for controller in self.slots.iter_mut().flatten() {
            if controller.despawn.is_some_and(|at| clock >= at) {
                gone.push(controller.uid);
                continue;
            }
            if let Some((interval, next)) = &mut controller.timer
                && clock >= *next
            {
                *next = clock + *interval;
                out.push((controller.uid, Due::Timer));
            }
            if controller.queue.iter().any(|&(at, _)| clock >= at) {
                let (now, later) = mem::take(&mut controller.queue)
                    .into_iter()
                    .partition(|&(at, _)| clock >= at);
                controller.queue = later;
                let now: Vec<(u32, i32)> = now;
                out.extend(
                    now.into_iter()
                        .map(|(_, arg)| (controller.uid, Due::Queue(arg))),
                );
            }
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
        let despawn = (duration > 0).then(|| self.clock + duration as u32);
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
            .add(coord, id, despawn, vars)
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
        delay: i32,
    ) -> Result<(), ScriptError> {
        let at = self.clock + delay.max(1) as u32;
        self.controller_mut(uid)?.queue.push((at, arg));
        Ok(())
    }

    pub(crate) fn controller_settimer(
        &mut self,
        uid: ControllerUid,
        interval: i32,
    ) -> Result<(), ScriptError> {
        let clock = self.clock;
        self.controller_mut(uid)?.timer =
            (interval > 0).then(|| (interval as u32, clock + interval as u32));
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

    /// Runs the controllers: deletes those whose time is up, then runs each
    /// one's due timer and queue scripts. A controller deleted by an earlier
    /// script this tick runs nothing more.
    pub(crate) fn process_controllers(&mut self) {
        let mut due = Vec::new();
        let mut gone = Vec::new();
        self.controllers.due(self.clock, &mut due, &mut gone);
        for uid in gone {
            self.controllers.remove(uid);
        }
        for (uid, work) in due {
            let Some(id) = self.controllers.get(uid).map(|controller| controller.id) else {
                continue;
            };
            let (trigger, last_int) = match work {
                Due::Timer => (ServerTriggerType::AiConTimer, None),
                Due::Queue(arg) => (ServerTriggerType::AiConQueue, Some(arg)),
            };
            let Some(script) = self.script_by_key(trigger, Some(id as i32), None) else {
                continue;
            };
            let mut state = self.build_state(script, None, None, None);
            state.active_controller = Some(uid);
            state.last_int = last_int;
            self.run_world_script(state);
        }
    }
}
