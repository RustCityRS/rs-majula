#![cfg(rev = "647")]

use rs_io::{crc, js5::Js5Store};
use rs_pack::archive;
use rs_pack::pack::{pack::pack_configs, packed_data::PackedData};
use rs_pack::unpack::unpack_configs_only;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::Path;

#[test]
#[ignore = "unpacks and repacks the original local 647 cache"]
fn original_configs_survive_unpack_and_repack() -> anyhow::Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    env::set_current_dir(root)?;
    let scratch = Path::new(env!("CARGO_TARGET_TMPDIR")).join("config_roundtrip_647");
    let packs = scratch.join("pack");
    let source = scratch.join("source");
    fs::create_dir_all(&packs)?;
    for entry in fs::read_dir(root.join(rs_pack::PACK_DIR))? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            fs::copy(entry.path(), packs.join(entry.file_name()))?;
        }
    }
    let expected = root.join("expected");
    unpack_configs_only(&expected, &source, &packs)?;
    let report = fs::read_to_string(source.join("unpack_report.txt"))?;
    assert!(report.contains("0 failed"), "{report}");

    // Compare to the original entries, without the authored server overlays.
    let packed = pack_configs(&source, &packs, false)?;
    let mut store = Js5Store::open(&expected)?;
    let mut failures = Vec::new();
    let mut total = 0;
    let mut kinds: Vec<_> = packed.keys().collect();
    kinds.sort();
    for kind in kinds {
        let Some(client) = &packed[kind].client else {
            continue;
        };
        let (aid, stride, groups) = match kind.as_str() {
            "loc" => (archive::CONFIG_LOC, 256, None),
            "enum" => (archive::CONFIG_ENUM, 256, None),
            "npc" => (archive::CONFIG_NPC, 128, None),
            "obj" => (archive::CONFIG_OBJ, 256, None),
            "seq" => (archive::CONFIG_SEQ, 128, None),
            "spotanim" => (archive::CONFIG_SPOTANIM, 256, None),
            "varpbit" => (archive::CONFIG_VARBIT, 1024, None),
            "billboard" => (archive::BILLBOARDS, 256, None),
            "particleemitter" => (archive::PARTICLES, 0, Some(0..1)),
            "particleeffector" => (archive::PARTICLES, 0, Some(1..2)),
            "quickchatcat" => (archive::QUICKCHAT, 0, Some(0..1)),
            "quickchatphrase" => (archive::QUICKCHAT, 0, Some(1..2)),
            _ => {
                let Some(&(_, group)) = archive::CONFIG_GROUPS
                    .iter()
                    .find(|(name, _)| *name == kind)
                else {
                    continue;
                };
                (archive::CONFIG, 0, Some(group..group + 1))
            }
        };
        let groups = groups.unwrap_or(0..store.group_count(aid).unwrap_or(0));
        let mut sources = vec![(aid, groups.clone(), 0)];
        if aid == archive::QUICKCHAT {
            sources.push((
                archive::QUICKCHAT_GLOBAL,
                groups,
                archive::QUICKCHAT_GLOBAL_FIRST as u32,
            ));
        }
        let mut original = BTreeMap::new();
        for (aid, groups, first) in sources {
            for gid in groups {
                if let Some(group) = store.read_group(aid, gid)? {
                    for entry in group.entries {
                        if !entry.data().is_empty() {
                            original.insert(
                                (first + gid * stride + entry.id) as u16,
                                entry.data().to_vec(),
                            );
                        }
                    }
                }
            }
        }
        let mut rebuilt: BTreeMap<_, _> = PackedData::entries(&client.dat, &client.idx)?
            .filter(|(_, data)| !data.is_empty())
            .map(|(id, data)| (id, data.to_vec()))
            .collect();
        let mut expected_bytes = Vec::new();
        let mut actual_bytes = Vec::new();
        let mut differing = Vec::new();
        for (&id, bytes) in &original {
            let actual = rebuilt.remove(&id).unwrap_or_default();
            expected_bytes.extend_from_slice(bytes);
            actual_bytes.extend_from_slice(&actual);
            if *bytes != actual {
                differing.push(id);
                if differing.len() <= 3 {
                    println!("{kind} {id}: expected {bytes:02x?}, actual {actual:02x?}");
                }
            }
        }
        differing.extend(rebuilt.keys());
        let expected_crc = crc::getcrc(&expected_bytes, 0, expected_bytes.len());
        let actual_crc = crc::getcrc(&actual_bytes, 0, actual_bytes.len());
        println!(
            "{kind}: {} records, CRC {expected_crc:#010x} -> {actual_crc:#010x}, {} differ",
            original.len(),
            differing.len()
        );
        total += original.len();
        if !differing.is_empty() {
            failures.push(format!(
                "{kind}: {} differing records, first {:?}",
                differing.len(),
                &differing[..differing.len().min(20)]
            ));
        }
    }
    assert!(total > 100_000, "only {total} original records checked");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    println!("{total} original config records match byte for byte");
    Ok(())
}
