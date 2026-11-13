use rs_grid::CoordGrid;
use rs_queue::ScriptQueue;
use rs_var::VarSet;
pub use rs_vm::ControllerUid;

/// An npc without a body: it stands on a coord, keeps its own vars and runs
/// its type's `[ai_contimer]` and its queued `[ai_conqueue]` and
/// `[controller_queue]` scripts.
pub struct Controller {
    pub uid: ControllerUid,
    pub id: u16,
    pub coord: CoordGrid,
    pub queues: ScriptQueue,
    pub timer_interval: Option<u16>,
    pub timer_clock: u16,
    pub lifecycle_at: Option<u32>,
    pub vars: VarSet,
}

impl Controller {
    pub fn new(
        uid: ControllerUid,
        id: u16,
        coord: CoordGrid,
        lifecycle_at: Option<u32>,
        vars: VarSet,
    ) -> Self {
        Self {
            uid,
            id,
            coord,
            queues: ScriptQueue::new(),
            timer_interval: None,
            timer_clock: 0,
            lifecycle_at,
            vars,
        }
    }
}
