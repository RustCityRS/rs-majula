#![cfg(since_647)]

use anyhow::Result;
use rs_io::Packet;
use rs_pack::cache::provider::CacheType;
use rs_pack::cache::spotanim::SpotAnimTypeRaw;
use rs_pack::pack::config::{
    bas, flo, loc, npc, particleeffector, particleemitter, quest, spotanim,
};
use rs_pack::pack::pack::FileCache;
use rs_pack::pack::pack_registry::{PackRegistry, PackedFile};
use rs_pack::pack::packed_data::PackedData;
use rs_pack::unpack::config::{
    UnpackedPacks, bas::emit_bas_props, decode_record, flo::emit_flo_props, loc::emit_loc_props,
    npc::emit_npc_props, particleeffector::emit_particleeffector_props,
    particleemitter::emit_particleemitter_props, quest::emit_quest_props,
    spotanim::emit_spotanim_props,
};
use std::collections::HashMap;
use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;

fn roundtrip<T: CacheType>(
    kind: &str,
    data: &[u8],
    expected: &str,
    legacy: &str,
    emit: impl Fn(&T, u8, &[u8], &mut UnpackedPacks, &HashMap<u16, u16>) -> Vec<(String, String)>,
    pack: impl Fn(&FileCache, &PackRegistry) -> Result<PackedFile>,
) {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("named_config_properties")
        .join(kind);
    fs::create_dir_all(&path).unwrap();
    fs::write(path.join(format!("{kind}.pack")), "0=sample\n").unwrap();
    fs::write(path.join("seq.pack"), "321=animation_321\n").unwrap();
    fs::write(path.join("texture.pack"), "7=material_seven\n").unwrap();
    let mut packs = UnpackedPacks::new();
    packs.load_existing_names(&path);
    let props = decode_record(
        &mut Packet::from(data.to_vec()),
        0,
        &HashMap::new(),
        &mut packs,
        emit,
    );
    let text: String = props
        .iter()
        .map(|(key, value)| format!("{key}={value}\n"))
        .collect();
    assert_eq!(text, expected, "{kind}");
    fs::write(
        path.join(format!("all.{kind}")),
        format!("[sample]\n{text}"),
    )
    .unwrap();
    let registry = PackRegistry::load(&path).unwrap();
    let packed = pack(&FileCache::new(&path), &registry)
        .unwrap()
        .client
        .unwrap();
    let bytes = PackedData::entries(&packed.dat, &packed.idx)
        .unwrap()
        .next()
        .unwrap()
        .1;
    assert_eq!(bytes, data, "{kind}: {text}");

    for property in legacy.lines() {
        fs::write(
            path.join(format!("all.{kind}")),
            format!("[sample]\n{property}\n"),
        )
        .unwrap();
        let result = catch_unwind(AssertUnwindSafe(|| pack(&FileCache::new(&path), &registry)));
        assert!(!matches!(result, Ok(Ok(_))), "{kind} accepted {property}");
    }
}

#[test]
fn identified_properties_preserve_encoding_and_reject_old_names() {
    roundtrip(
        "bas",
        &[45, 1, 65, 0],
        "overlayheight=321\n",
        "unknown45=animation_321\n",
        emit_bas_props,
        |files, registry| bas::pack_bases(files, registry, &HashMap::new()),
    );
    roundtrip(
        "flo",
        &[2, 7, 10, 0],
        "texture=material_seven\nhardshadow=no\n",
        "unknown2=7\nunknown10=yes\n",
        emit_flo_props,
        |files, registry| flo::pack_flos(files, registry, &HashMap::new()),
    );
    roundtrip(
        "loc",
        &[105, 164, 255, 254, 165, 0, 3, 166, 128, 0, 0],
        "msimirror=yes\npostoffsetx=-2\npostoffsety=3\npostoffsetz=-32768\n",
        "unknown105=yes\nunknown164=-2\nunknown165=3\nunknown166=-32768\n",
        emit_loc_props,
        |files, registry| loc::pack_locs(files, registry, &HashMap::new(), &HashMap::new()),
    );
    roundtrip(
        "npc",
        &[165, 7, 0],
        "picksizeshift=7\n",
        "unknown165=7\n",
        emit_npc_props,
        |files, registry| npc::pack_npcs(files, registry, &HashMap::new(), &HashMap::new()),
    );
    roundtrip(
        "particleeffector",
        &[10, 0],
        "reversedirection=yes\n",
        "unknown10=yes\n",
        emit_particleeffector_props,
        |files, registry| {
            particleeffector::pack_particleeffectors(files, registry, &HashMap::new())
        },
    );
    roundtrip(
        "particleemitter",
        &[25, 2, 0, 7, 1, 1, 0],
        "globaleffectors=7,257\n",
        "unknown25=7,257\n",
        emit_particleemitter_props,
        |files, registry| particleemitter::pack_particleemitters(files, registry, &HashMap::new()),
    );
    roundtrip(
        "quest",
        &[12, 0x10, 0x01, 0, 5, 0],
        "viacoord=1_0_0_4_5\n",
        "unknown12=268500997\n",
        emit_quest_props,
        |files, registry| quest::pack_quests(files, registry, &HashMap::new(), &HashMap::new()),
    );
    roundtrip(
        "spotanim",
        &[
            9, 11, 12, 13, 14, 5, 15, 0, 7, 16, 0, 0, 0, 7, 16, 255, 255, 255, 255, 0,
        ],
        "hillchange=rotate\nhillchange=skew\nhillchange=ceiling_skew\nhillchange=skew_to_fit\n\
         hillchange=tree_skew,5\nhillchange=rotate,7\nhillchange=rotate,7,32\nhillchange=rotate,-1\n",
        "unknown9=yes\nunknown11=yes\nunknown12=yes\nunknown13=yes\nunknown14=5\nunknown15=7\nunknown16=7\nunknown16=-1\n",
        emit_spotanim_props,
        |files, registry| spotanim::pack_spotanims(files, registry, &HashMap::new()),
    );
}

#[test]
fn spotanim_terrain_mode_uses_client_defaults_and_overwrites_previous_modes() {
    let mut spot = SpotAnimTypeRaw::new(0);
    assert_eq!((spot.hillskew, spot.hillskew_param), (0, None));
    spot.decode(&mut Packet::from(vec![9, 0]));
    assert_eq!((spot.hillskew, spot.hillskew_param), (3, Some(8224)));
    spot.decode(&mut Packet::from(vec![14, 5, 12, 0]));
    assert_eq!((spot.hillskew, spot.hillskew_param), (4, Some(1280)));
    spot.decode(&mut Packet::from(vec![16, 255, 255, 255, 255, 0]));
    assert_eq!((spot.hillskew, spot.hillskew_param), (3, None));
}
