type Ops<'a> = Option<&'a [Option<Box<str>>]>;

/// Whether the client shows option `i` (from 0): it hides "hidden" ones.
fn shown(ops: Ops, i: usize) -> bool {
    name(ops, i).is_some()
}

fn name(ops: Ops<'_>, i: usize) -> Option<&str> {
    ops.and_then(|ops| ops.get(i))
        .and_then(Option::as_deref)
        .filter(|op| !op.eq_ignore_ascii_case("hidden"))
}

/// Whether NPC option `op` (1-5) can be the left click. Attack comes first,
/// unless the NPC is a higher level than the player.
pub fn npc(ops: Ops, op: u8, npc_level: i32, player_level: i32) -> bool {
    let attack = |i| name(ops, i).is_some_and(|op| op.eq_ignore_ascii_case("attack"));
    let first_attack = (npc_level <= player_level)
        .then(|| (0..5).find(|&i| attack(i)))
        .flatten();
    let top = first_attack.or_else(|| (0..5).find(|&i| shown(ops, i) && !attack(i)));
    top == Some(usize::from(op) - 1)
}

/// Whether loc option `op` (1-5) can be the left click: the first one does.
pub fn loc(ops: Ops, op: u8) -> bool {
    (0..5).find(|&i| shown(ops, i)) == Some(usize::from(op) - 1)
}

/// Whether option `op` (1-5) on an obj on the ground can be the left click.
/// The third is "Take" when the obj has none of its own.
pub fn ground_obj(ops: Ops, op: u8) -> bool {
    (0..5).find(|&i| i == 2 || shown(ops, i)) == Some(usize::from(op) - 1)
}

/// An option on an obj in an inventory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvOp {
    Interface(usize),
    Obj(usize),
    Use,
}

/// The option a left click on an obj in an inventory picks. `operable`
/// interfaces show the obj's options, `usable` ones "Use".
pub fn inv_top(interface_ops: Ops, operable: bool, usable: bool, obj_ops: Ops) -> InvOp {
    if let Some(i) = (0..5).find(|&i| shown(interface_ops, i)) {
        return InvOp::Interface(i);
    }
    if operable && let Some(i) = (0..3).find(|&i| shown(obj_ops, i)) {
        return InvOp::Obj(i);
    }
    if usable {
        return InvOp::Use;
    }
    if operable && shown(obj_ops, 3) {
        return InvOp::Obj(3);
    }
    InvOp::Obj(4)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ops(names: &[Option<&str>]) -> Box<[Option<Box<str>>]> {
        names.iter().map(|n| n.map(Box::from)).collect()
    }

    #[test]
    fn npcs_attack_first_unless_above_the_player() {
        let man = ops(&[Some("Talk-to"), Some("Attack"), Some("Pickpocket")]);
        assert!(npc(Some(&man), 2, 2, 3));
        assert!(!npc(Some(&man), 1, 2, 3));
        // Above the player: Attack sinks below "Walk here".
        assert!(npc(Some(&man), 1, 5, 3));
        assert!(!npc(Some(&man), 2, 5, 3));
        assert!(!npc(Some(&man), 3, 2, 3));
        let guard = ops(&[None, Some("Attack")]);
        assert!(!npc(Some(&guard), 2, 21, 3));
        let banker = ops(&[Some("Talk-to"), None, Some("Bank")]);
        assert!(npc(Some(&banker), 1, 0, 3));
        assert!(!npc(Some(&banker), 3, 0, 3));
    }

    #[test]
    fn locs_and_ground_objs_left_click_their_first_option() {
        let booth = ops(&[Some("hidden"), Some("Use-quickly")]);
        assert!(loc(Some(&booth), 2));
        assert!(!loc(Some(&ops(&[Some("Open"), Some("Search")])), 2));
        assert!(ground_obj(None, 3));
        assert!(!ground_obj(None, 1));
        let light = ops(&[None, None, None, Some("Light")]);
        assert!(ground_obj(Some(&light), 3));
        assert!(!ground_obj(Some(&light), 4));
    }

    #[test]
    fn inventory_left_clicks() {
        let food = ops(&[Some("Eat")]);
        assert_eq!(inv_top(None, true, true, Some(&food)), InvOp::Obj(0));
        assert_eq!(inv_top(None, true, true, None), InvOp::Use);
        assert_eq!(inv_top(None, true, false, None), InvOp::Obj(4));
        let rune = ops(&[None, None, None, Some("Rub")]);
        assert_eq!(inv_top(None, true, false, Some(&rune)), InvOp::Obj(3));
        let bank = ops(&[Some("Withdraw-1"), Some("Withdraw-5")]);
        assert_eq!(
            inv_top(Some(&bank), false, false, Some(&food)),
            InvOp::Interface(0)
        );
    }
}
