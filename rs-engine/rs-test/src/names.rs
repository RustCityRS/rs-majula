//! Interface and component ids by their names in the rev's `pack/interface.pack`, so a test
//! reaches the same widget on every rev whose content names it.

use std::collections::HashMap;
use std::sync::LazyLock;

static IDS: LazyLock<HashMap<String, i32>> = LazyLock::new(|| {
    let path = format!(
        "{}/../../{}/pack/interface.pack",
        env!("CARGO_MANIFEST_DIR"),
        rs_pack::CONTENT_DIR
    );
    let pack = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let lines: Vec<(&str, &str)> = pack.lines().filter_map(|l| l.split_once('=')).collect();
    // 647's pack names some interfaces after their components.
    let interfaces: HashMap<i32, &str> = lines
        .iter()
        .filter(|(key, _)| !key.contains(':'))
        .map(|&(key, name)| (key.parse().expect("interface id"), name))
        .collect();
    let mut ids: HashMap<String, i32> = interfaces
        .iter()
        .map(|(&id, &name)| (name.to_owned(), id))
        .collect();
    for (key, name) in lines {
        let Some((interface, com)) = key.split_once(':') else {
            continue;
        };
        let interface: i32 = interface.parse().expect("interface id");
        let com: i32 = com.parse().expect("component id");
        let parent = interfaces[&interface];
        ids.insert(format!("{parent}:{name}"), (interface << 16) | com);
    }
    ids
});

/// The interface named `name`.
pub fn iface(name: &str) -> u16 {
    *IDS.get(name)
        .unwrap_or_else(|| panic!("no interface {name} in {}", rs_pack::CONTENT_DIR)) as u16
}

/// The component named `interface:component`, packed `interface << 16 | component`.
pub fn com(name: &str) -> i32 {
    *IDS.get(name)
        .unwrap_or_else(|| panic!("no component {name} in {}", rs_pack::CONTENT_DIR))
}

/// The backpack interface, `inventory`.
#[cfg(all(since_530, before_742))]
pub const INVENTORY: u16 = 149;
#[cfg(since_742)]
pub const INVENTORY: u16 = 679;

/// The backpack's slots, `inventory:inv`.
pub const BACKPACK: i32 = (INVENTORY as i32) << 16;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_backpack_is_named_inventory_inv() {
        assert_eq!(iface("inventory"), INVENTORY);
        assert_eq!(com("inventory:inv"), BACKPACK);
    }
}
