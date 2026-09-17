use num_enum::TryFromPrimitive;

/// Enumerates all server-side script trigger types that can initiate script execution.
///
/// Each variant corresponds to a specific game event or interaction that causes the
/// engine to look up and execute a script. The discriminant values (`u8`) are used
/// as part of the trigger lookup key in the engine's script trigger table.
///
/// Trigger naming conventions:
/// - `Ap*` -- "approach" triggers, fired when a player walks toward an entity.
/// - `Op*` -- "option" triggers, fired when a player clicks an entity menu option (1-5).
/// - `*U` -- "use item on" variant of the trigger.
/// - `*T` -- "spell/target" variant of the trigger.
/// - `Ai*` -- NPC AI-initiated versions of the corresponding triggers.
/// - `AiQueue*` -- Queued AI script triggers (1-20 priority levels).
///
/// The engine builds lookup keys as: `trigger_type | (0x1_or_0x2 << 8) | (entity_id << 10)`.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
pub enum ServerTriggerType {
    Proc = 0,
    Label = 1,
    DebugProc = 2,

    ApNpc1 = 3,
    ApNpc2 = 4,
    ApNpc3 = 5,
    ApNpc4 = 6,
    ApNpc5 = 7,
    ApNpcU = 8,
    ApNpcT = 9,
    OpNpc1 = 10,
    OpNpc2 = 11,
    OpNpc3 = 12,
    OpNpc4 = 13,
    OpNpc5 = 14,
    OpNpcU = 15,
    OpNpcT = 16,
    AiApNpc1 = 17,
    AiApNpc2 = 18,
    AiApNpc3 = 19,
    AiApNpc4 = 20,
    AiApNpc5 = 21,
    AiOpNpc1 = 24,
    AiOpNpc2 = 25,
    AiOpNpc3 = 26,
    AiOpNpc4 = 27,
    AiOpNpc5 = 28,

    ApObj1 = 31,
    ApObj2 = 32,
    ApObj3 = 33,
    ApObj4 = 34,
    ApObj5 = 35,
    ApObjU = 36,
    ApObjT = 37,
    OpObj1 = 38,
    OpObj2 = 39,
    OpObj3 = 40,
    OpObj4 = 41,
    OpObj5 = 42,
    OpObjU = 43,
    OpObjT = 44,
    AiApObj1 = 45,
    AiApObj2 = 46,
    AiApObj3 = 47,
    AiApObj4 = 48,
    AiApObj5 = 49,
    AiOpObj1 = 52,
    AiOpObj2 = 53,
    AiOpObj3 = 54,
    AiOpObj4 = 55,
    AiOpObj5 = 56,

    ApLoc1 = 59,
    ApLoc2 = 60,
    ApLoc3 = 61,
    ApLoc4 = 62,
    ApLoc5 = 63,
    ApLocU = 64,
    ApLocT = 65,
    OpLoc1 = 66,
    OpLoc2 = 67,
    OpLoc3 = 68,
    OpLoc4 = 69,
    OpLoc5 = 70,
    OpLocU = 71,
    OpLocT = 72,
    AiApLoc1 = 73,
    AiApLoc2 = 74,
    AiApLoc3 = 75,
    AiApLoc4 = 76,
    AiApLoc5 = 77,
    AiOpLoc1 = 80,
    AiOpLoc2 = 81,
    AiOpLoc3 = 82,
    AiOpLoc4 = 83,
    AiOpLoc5 = 84,

    ApPlayer1 = 87,
    ApPlayer2 = 88,
    ApPlayer3 = 89,
    ApPlayer4 = 90,
    ApPlayer5 = 91,
    ApPlayerU = 92,
    ApPlayerT = 93,
    OpPlayer1 = 94,
    OpPlayer2 = 95,
    OpPlayer3 = 96,
    OpPlayer4 = 97,
    OpPlayer5 = 98,
    OpPlayerU = 99,
    OpPlayerT = 100,
    AiApPlayer1 = 101,
    AiApPlayer2 = 102,
    AiApPlayer3 = 103,
    AiApPlayer4 = 104,
    AiApPlayer5 = 105,
    AiOpPlayer1 = 108,
    AiOpPlayer2 = 109,
    AiOpPlayer3 = 110,
    AiOpPlayer4 = 111,
    AiOpPlayer5 = 112,

    Queue = 116,
    AiQueue1 = 117,
    AiQueue2 = 118,
    AiQueue3 = 119,
    AiQueue4 = 120,
    AiQueue5 = 121,
    AiQueue6 = 122,
    AiQueue7 = 123,
    AiQueue8 = 124,
    AiQueue9 = 125,
    AiQueue10 = 126,
    AiQueue11 = 127,
    AiQueue12 = 128,
    AiQueue13 = 129,
    AiQueue14 = 130,
    AiQueue15 = 131,
    AiQueue16 = 132,
    AiQueue17 = 133,
    AiQueue18 = 134,
    AiQueue19 = 135,
    AiQueue20 = 136,

    SoftTimer = 137,
    Timer = 138,
    AiTimer = 139,

    OpHeld1 = 140,
    OpHeld2 = 141,
    OpHeld3 = 142,
    OpHeld4 = 143,
    OpHeld5 = 144,
    OpHeldU = 145,
    OpHeldT = 146,

    IfButton = 147,
    IfClose = 148,
    InvButton1 = 149,
    InvButton2 = 150,
    InvButton3 = 151,
    InvButton4 = 152,
    InvButton5 = 153,
    InvButtonD = 154,

    WalkTrigger = 155,
    AiWalkTrigger = 156,

    Login = 157,
    Logout = 158,
    Tutorial = 159,
    AdvanceStat = 160,
    MapZone = 161,
    MapZoneExit = 162,
    Zone = 163,
    ZoneExit = 164,
    ChangeStat = 165,
    AiSpawn = 166,
    AiDespawn = 167,
}

impl ServerTriggerType {
    /// Returns whether this trigger type provides `last_use` (the item used on the target).
    ///
    /// Only `*U` (use-item-on) trigger variants return `true`. Scripts accessing
    /// `last_useitem` or `last_useslot` require a trigger type that passes this check.
    ///
    /// # Returns
    /// `true` for all `Op*U` and `Ap*U` triggers, `false` otherwise.
    #[allow(unused)]
    pub(crate) fn allows_last_use(self) -> bool {
        matches!(
            self,
            Self::OpHeldU
                | Self::ApObjU
                | Self::ApLocU
                | Self::ApNpcU
                | Self::ApPlayerU
                | Self::OpObjU
                | Self::OpLocU
                | Self::OpNpcU
                | Self::OpPlayerU
        )
    }

    /// Returns whether this trigger type provides `last_slot` (the inventory slot interacted with).
    ///
    /// Only held-item (`OpHeld*`) and inventory button (`InvButton*`) trigger variants
    /// return `true`. Scripts accessing `last_slot` require a trigger that passes this check.
    ///
    /// # Returns
    /// `true` for all `OpHeld*` and `InvButton*` triggers, `false` otherwise.
    pub(crate) fn allows_last_slot(self) -> bool {
        matches!(
            self,
            Self::OpHeld1
                | Self::OpHeld2
                | Self::OpHeld3
                | Self::OpHeld4
                | Self::OpHeld5
                | Self::OpHeldU
                | Self::OpHeldT
                | Self::InvButton1
                | Self::InvButton2
                | Self::InvButton3
                | Self::InvButton4
                | Self::InvButton5
                | Self::InvButtonD
        )
    }

    /// Returns whether this trigger type provides `last_item` (the item in the interacted slot).
    ///
    /// Similar to [`allows_last_slot`](Self::allows_last_slot) but excludes `InvButtonD`
    /// (the drag/drop trigger which does not have a specific source item).
    ///
    /// # Returns
    /// `true` for all `OpHeld*` and `InvButton1`-`InvButton5` triggers, `false` otherwise.
    pub(crate) fn allows_last_item(self) -> bool {
        matches!(
            self,
            Self::OpHeld1
                | Self::OpHeld2
                | Self::OpHeld3
                | Self::OpHeld4
                | Self::OpHeld5
                | Self::OpHeldU
                | Self::OpHeldT
                | Self::InvButton1
                | Self::InvButton2
                | Self::InvButton3
                | Self::InvButton4
                | Self::InvButton5
        )
    }

    /// Returns whether this trigger type provides `last_targetslot` (the drag destination slot).
    ///
    /// Only the `InvButtonD` (inventory drag/drop) trigger returns `true`, as it is
    /// the only interaction with both a source and target slot.
    ///
    /// # Returns
    /// `true` only for `InvButtonD`, `false` for all other triggers.
    pub(crate) fn allows_last_targetslot(self) -> bool {
        matches!(self, Self::InvButtonD)
    }

    /// Returns whether this trigger type provides `last_useitem` (the item that was used).
    ///
    /// Identical set to [`allows_last_use`](Self::allows_last_use) -- all `*U` triggers.
    /// The `last_useitem` is the item ID of the item being used on the target entity.
    ///
    /// # Returns
    /// `true` for all `Op*U` and `Ap*U` triggers, `false` otherwise.
    pub(crate) fn allows_last_useitem(self) -> bool {
        matches!(
            self,
            Self::OpHeldU
                | Self::ApObjU
                | Self::ApLocU
                | Self::ApNpcU
                | Self::ApPlayerU
                | Self::OpObjU
                | Self::OpLocU
                | Self::OpNpcU
                | Self::OpPlayerU
        )
    }

    /// Returns whether this trigger type provides `last_useslot` (the inventory slot of the used item).
    ///
    /// Identical set to [`allows_last_use`](Self::allows_last_use) -- all `*U` triggers.
    /// The `last_useslot` is the inventory slot index the used item occupies.
    ///
    /// # Returns
    /// `true` for all `Op*U` and `Ap*U` triggers, `false` otherwise.
    pub(crate) fn allows_last_useslot(self) -> bool {
        matches!(
            self,
            Self::OpHeldU
                | Self::ApObjU
                | Self::ApLocU
                | Self::ApNpcU
                | Self::ApPlayerU
                | Self::OpObjU
                | Self::OpLocU
                | Self::OpNpcU
                | Self::OpPlayerU
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_from_valid_values() {
        assert_eq!(
            ServerTriggerType::try_from(0u8).unwrap(),
            ServerTriggerType::Proc
        );
        assert_eq!(
            ServerTriggerType::try_from(1u8).unwrap(),
            ServerTriggerType::Label
        );
        assert_eq!(
            ServerTriggerType::try_from(2u8).unwrap(),
            ServerTriggerType::DebugProc
        );
    }

    #[test]
    fn try_from_invalid_value() {
        assert!(ServerTriggerType::try_from(250u8).is_err());
    }

    #[test]
    fn allows_last_use_true_cases() {
        let triggers = [
            ServerTriggerType::OpHeldU,
            ServerTriggerType::ApObjU,
            ServerTriggerType::ApLocU,
            ServerTriggerType::ApNpcU,
            ServerTriggerType::ApPlayerU,
            ServerTriggerType::OpObjU,
            ServerTriggerType::OpLocU,
            ServerTriggerType::OpNpcU,
            ServerTriggerType::OpPlayerU,
        ];
        for t in triggers {
            assert!(t.allows_last_use(), "{:?} should allow last use", t);
        }
    }

    #[test]
    fn allows_last_use_false_cases() {
        let triggers = [
            ServerTriggerType::Proc,
            ServerTriggerType::Label,
            ServerTriggerType::OpNpc1,
            ServerTriggerType::Queue,
            ServerTriggerType::Timer,
            ServerTriggerType::Login,
        ];
        for t in triggers {
            assert!(!t.allows_last_use(), "{:?} should not allow last use", t);
        }
    }

    #[test]
    fn allows_last_slot_true_cases() {
        let triggers = [
            ServerTriggerType::OpHeld1,
            ServerTriggerType::OpHeld2,
            ServerTriggerType::OpHeld3,
            ServerTriggerType::OpHeld4,
            ServerTriggerType::OpHeld5,
            ServerTriggerType::OpHeldU,
            ServerTriggerType::OpHeldT,
            ServerTriggerType::InvButton1,
            ServerTriggerType::InvButton2,
            ServerTriggerType::InvButton3,
            ServerTriggerType::InvButton4,
            ServerTriggerType::InvButton5,
            ServerTriggerType::InvButtonD,
        ];
        for t in triggers {
            assert!(t.allows_last_slot(), "{:?} should allow last slot", t);
        }
    }

    #[test]
    fn allows_last_slot_false_cases() {
        let triggers = [
            ServerTriggerType::Proc,
            ServerTriggerType::Label,
            ServerTriggerType::OpNpc1,
            ServerTriggerType::ApLoc1,
            ServerTriggerType::Queue,
            ServerTriggerType::Timer,
            ServerTriggerType::Login,
            ServerTriggerType::IfButton,
        ];
        for t in triggers {
            assert!(!t.allows_last_slot(), "{:?} should not allow last slot", t);
        }
    }

    #[test]
    fn repr_values() {
        assert_eq!(ServerTriggerType::Proc as u8, 0);
        assert_eq!(ServerTriggerType::Label as u8, 1);
        assert_eq!(ServerTriggerType::DebugProc as u8, 2);
        assert_eq!(ServerTriggerType::Queue as u8, 116);
        assert_eq!(ServerTriggerType::Timer as u8, 138);
        assert_eq!(ServerTriggerType::Login as u8, 157);
        assert_eq!(ServerTriggerType::AiDespawn as u8, 167);
    }

    #[allow(clippy::clone_on_copy)]
    #[test]
    fn clone_and_copy() {
        let a = ServerTriggerType::Proc;
        let b = a;
        let c = a.clone();
        assert_eq!(a, b);
        assert_eq!(a, c);
    }

    #[test]
    fn npc_trigger_values() {
        assert_eq!(ServerTriggerType::ApNpc1 as u8, 3);
        assert_eq!(ServerTriggerType::OpNpc5 as u8, 14);
        assert_eq!(ServerTriggerType::AiApNpc1 as u8, 17);
        assert_eq!(ServerTriggerType::AiOpNpc1 as u8, 24);
    }

    #[test]
    fn trigger_lookup_key_calculation() {
        // Engine builds: base | (0x2 << 8) | (type << 10)
        let trigger = ServerTriggerType::OpNpc1;
        let base = trigger as i32;
        let id = 42u16;
        let key = base | (0x2 << 8) | ((id as i32) << 10);
        assert_eq!(key & 0xFF, base);
        assert_eq!((key >> 10) as u16, id);
    }

    #[test]
    fn trigger_lookup_category_key() {
        let trigger = ServerTriggerType::OpLoc1;
        let base = trigger as i32;
        let category = 5i32;
        let key = base | (0x1 << 8) | (category << 10);
        assert_eq!(key & 0xFF, base);
        assert_eq!(key >> 10, category);
    }

    #[test]
    fn all_u_triggers_allow_last_use() {
        let u_triggers = [
            ServerTriggerType::OpHeldU,
            ServerTriggerType::ApObjU,
            ServerTriggerType::ApLocU,
            ServerTriggerType::ApNpcU,
            ServerTriggerType::ApPlayerU,
            ServerTriggerType::OpObjU,
            ServerTriggerType::OpLocU,
            ServerTriggerType::OpNpcU,
            ServerTriggerType::OpPlayerU,
        ];
        for t in u_triggers {
            assert!(t.allows_last_use(), "{:?} should allow last_use", t);
        }
    }

    #[test]
    fn non_u_triggers_dont_allow_last_use() {
        // Numbered triggers (Op1-5, Ap1-5) should not
        assert!(!ServerTriggerType::OpHeld1.allows_last_use());
        assert!(!ServerTriggerType::OpHeld5.allows_last_use());
        assert!(!ServerTriggerType::OpHeldT.allows_last_use());
    }

    #[test]
    fn all_inv_button_triggers_allow_last_slot() {
        let inv_triggers = [
            ServerTriggerType::InvButton1,
            ServerTriggerType::InvButton2,
            ServerTriggerType::InvButton3,
            ServerTriggerType::InvButton4,
            ServerTriggerType::InvButton5,
            ServerTriggerType::InvButtonD,
        ];
        for t in inv_triggers {
            assert!(t.allows_last_slot(), "{:?} should allow last_slot", t);
        }
    }

    #[test]
    fn player_triggers_dont_allow_last_slot() {
        assert!(!ServerTriggerType::OpPlayer1.allows_last_slot());
        assert!(!ServerTriggerType::ApPlayer1.allows_last_slot());
    }

    #[test]
    fn walk_triggers() {
        assert_eq!(ServerTriggerType::WalkTrigger as u8, 155);
        assert_eq!(ServerTriggerType::AiWalkTrigger as u8, 156);
        assert!(!ServerTriggerType::WalkTrigger.allows_last_slot());
        assert!(!ServerTriggerType::WalkTrigger.allows_last_use());
    }

    #[test]
    fn event_trigger_values() {
        assert_eq!(ServerTriggerType::Login as u8, 157);
        assert_eq!(ServerTriggerType::Logout as u8, 158);
        assert_eq!(ServerTriggerType::Tutorial as u8, 159);
        assert_eq!(ServerTriggerType::AdvanceStat as u8, 160);
    }
}
