#![cfg(since_647)]

use rs_io::Packet;
use rs_pack::cache::ScriptVarType;
use rs_pack::cache::controller::ControllerType;
use rs_pack::cache::provider::CacheType;
use rs_pack::cache::script::{POP_VARCONBIT, PUSH_VARCONBIT, ScriptProvider};
use rs_pack::cache::varbit::VarbitType;
use rs_pack::cache::varp::VarPlayerTypeRaw;
use rs_pack::pack::config::controller::pack_controllers;
use rs_pack::pack::config::varbit::pack_varbits;
use rs_pack::pack::config::varconbit::pack_varconbits;
use rs_pack::pack::config::varp::pack_varps;
use rs_pack::pack::config::vars::{pack_string_vars, pack_vars};
use rs_pack::pack::pack::FileCache;
use rs_pack::pack::pack_registry::PackRegistry;
use rs_pack::pack::packed_data::PackedData;
use runec::compile_memory;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

fn entries(data: &PackedData) -> Vec<Vec<u8>> {
    PackedData::entries(&data.dat, &data.idx)
        .unwrap()
        .map(|(_, data)| data.to_vec())
        .collect()
}

#[test]
fn shared_variable_files_route_strings_and_controller_bits() {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join("variable-configs-647");
    let scripts = path.join("scripts");
    let pack = path.join("pack");
    fs::create_dir_all(&scripts).unwrap();
    fs::create_dir_all(&pack).unwrap();
    for (kind, text) in [
        ("varp", "// client_count=1\n0=player_int\n1=player_string\n"),
        ("varpstr", "// client_count=1\n0=player_string\n"),
        ("vars", "// client_count=1\n0=world_int\n1=world_string\n"),
        ("varsstr", "// client_count=1\n0=world_string\n"),
        ("varpbit", "0=player_bits\n"),
        ("varcon", "0=controller_int\n"),
        ("varconbit", "0=controller_bits\n"),
        ("controller", "0=example\n"),
    ] {
        fs::write(pack.join(format!("{kind}.pack")), text).unwrap();
    }
    for (kind, text) in [
        (
            "varp",
            "[player_int]\nclientcode=4\n[player_string]\ntype=string\n",
        ),
        ("vars", "[world_int]\n[world_string]\ntype=string\n"),
        (
            "varpbit",
            "[player_bits]\nbasevar=player_int\nstartbit=4\nendbit=7\n",
        ),
        ("varcon", "[controller_int]\ntype=int\n"),
        (
            "varconbit",
            "[controller_bits]\nbasevar=controller_int\nstartbit=4\nendbit=7\n",
        ),
        (
            "controller",
            "[example]\nvarcon=controller_int,9\nvarconbit=controller_bits,3\n",
        ),
    ] {
        fs::write(scripts.join(format!("all.{kind}")), text).unwrap();
    }
    let registry = PackRegistry::load(&pack).unwrap();
    let files = FileCache::new(&scripts);
    let constants = HashMap::new();
    registry.check_client_ids().unwrap();
    registry.check_content(&files).unwrap();

    let varps = pack_varps(&files, &registry, &constants).unwrap();
    assert_eq!(entries(varps.client.as_ref().unwrap()), [vec![5, 0, 4, 0]]);
    let mut string = VarPlayerTypeRaw::new(1);
    string.decode(&mut Packet::from(entries(&varps.server)[1].clone()));
    assert_eq!(string.var_type, ScriptVarType::String);
    let vars = pack_vars(&files, &registry.vars, "vars", &constants).unwrap();
    assert_eq!(entries(vars.client.as_ref().unwrap()), [vec![0]]);
    for (kind, pack) in [("varp", &registry.varpstr), ("vars", &registry.varsstr)] {
        let strings = pack_string_vars(&files, pack, kind, &constants).unwrap();
        assert_eq!(entries(strings.client.as_ref().unwrap()), [vec![0]]);
    }

    let player_bits = pack_varbits(&files, &registry, &constants).unwrap();
    assert_eq!(
        entries(player_bits.client.as_ref().unwrap()),
        [vec![1, 0, 0, 4, 7, 0]]
    );
    let bits = pack_varconbits(&files, &registry, &constants).unwrap();
    assert_eq!(&entries(&bits)[0][..5], &[1, 0, 0, 4, 7]);
    let mut bit = VarbitType::new(0);
    bit.decode(&mut Packet::from(entries(&bits)[0].clone()));
    assert_eq!((bit.basevar, bit.start_bit, bit.end_bit), (0, 4, 7));
    let controllers = pack_controllers(&files, &registry, &constants).unwrap();
    let mut controller = ControllerType::new(0);
    controller.decode(&mut Packet::from(entries(&controllers)[0].clone()));
    assert_eq!(controller.varbits.as_ref(), &[(0, 3)]);

    fs::write(
        scripts.join("bits.rs2"),
        "[proc,controller_bit_access]()(int)\n%controller_bits = 5;\n.%controller_bits = 3;\nreturn(.%controller_bits);\n",
    ).unwrap();
    let (dat, idx) = compile_memory(&scripts, Some(&pack), false).unwrap();
    let scripts = ScriptProvider::from_bytes(&dat, &idx);
    let script = scripts
        .scripts
        .iter()
        .flatten()
        .find(|script| script.opcodes.contains(&POP_VARCONBIT))
        .unwrap();
    let ops: Vec<_> = script
        .opcodes
        .iter()
        .copied()
        .zip(script.int_operands.iter().copied())
        .filter(|(op, _)| matches!(*op, PUSH_VARCONBIT | POP_VARCONBIT))
        .collect();
    assert_eq!(
        ops,
        [
            (POP_VARCONBIT, 0),
            (POP_VARCONBIT, 1 << 16),
            (PUSH_VARCONBIT, 1 << 16)
        ]
    );
}
