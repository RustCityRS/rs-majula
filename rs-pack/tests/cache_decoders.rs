use rs_io::Packet;
use rs_pack::ParamValue;
#[cfg(since_647)]
use rs_pack::cache::bas::BasType;
#[cfg(since_647)]
#[cfg(since_647)]
#[cfg(since_647)]
use rs_pack::cache::billboard::BillboardType;
#[cfg(since_647)]
use rs_pack::cache::dbrow::{DbRowType, DbRowValue};
#[cfg(since_647)]
use rs_pack::cache::r#enum::{EnumType, EnumTypeRaw};
#[cfg(since_647)]
use rs_pack::cache::flo::{FloType, FloTypeRaw};
#[cfg(since_647)]
use rs_pack::cache::flu::FluType;
use rs_pack::cache::hunt::HuntType;
#[cfg(since_647)]
use rs_pack::cache::idk::IdkTypeRaw;
use rs_pack::cache::inv::InvTypeRaw;
use rs_pack::cache::loc::LocTypeRaw;
#[cfg(since_647)]
use rs_pack::cache::map::load_xtea_keys;
use rs_pack::cache::npc::NpcTypeRaw;
use rs_pack::cache::obj::ObjTypeRaw;
#[cfg(since_647)]
use rs_pack::cache::obj::{ObjContext, ObjType};
#[cfg(since_647)]
use rs_pack::cache::param::{ParamType, ParamTypeRaw};
#[cfg(since_647)]
use rs_pack::cache::particleeffector::ParticleEffectorType;
#[cfg(since_647)]
use rs_pack::cache::particleemitter::ParticleEmitterType;
use rs_pack::cache::provider::CacheType;
#[cfg(since_647)]
use rs_pack::cache::provider::TypeProvider;
use rs_pack::cache::seq::SeqTypeRaw;
#[cfg(since_647)]
use rs_pack::cache::spotanim::SpotAnimTypeRaw;
use rs_pack::cache::r#struct::{StructType, StructTypeRaw};
#[cfg(since_647)]
use rs_pack::pack::config::bas::pack_bases;
#[cfg(since_647)]
use rs_pack::pack::config::billboard::pack_billboards;
#[cfg(since_647)]
use rs_pack::pack::config::dbrow::pack_dbrows;
use rs_pack::pack::config::hunt::pack_hunts;
use rs_pack::pack::config::npc::pack_npcs;
#[cfg(since_647)]
use rs_pack::pack::config::pack_empty_table;
#[cfg(before_647)]
use rs_pack::pack::config::param::parse_params;
#[cfg(since_647)]
use rs_pack::pack::config::particleeffector::pack_particleeffectors;
#[cfg(since_647)]
use rs_pack::pack::config::particleemitter::pack_particleemitters;
#[cfg(since_647)]
use rs_pack::pack::config::varc::{pack_varc_strings, pack_varcs};
#[cfg(since_647)]
use rs_pack::pack::config::varclansetting::pack_varclansettings;
use rs_pack::pack::pack::FileCache;
use rs_pack::pack::pack_registry::PackRegistry;
use rs_pack::pack::packed_data::PackedData;
use rs_pack::types::HuntCheckAfk;
#[cfg(since_647)]
use rs_pack::types::HuntModeType;
#[cfg(since_647)]
use rs_pack::types::{ScriptVarType, WearPos};
#[cfg(since_647)]
use rs_pack::unpack::config::bas::emit_bas_props;
#[cfg(since_647)]
use rs_pack::unpack::config::billboard::emit_billboard_props;
#[cfg(since_647)]
use rs_pack::unpack::config::particleeffector::emit_particleeffector_props;
#[cfg(since_647)]
use rs_pack::unpack::config::particleemitter::emit_particleemitter_props;
#[cfg(since_647)]
use rs_pack::unpack::config::{
    UnpackedPacks, r#enum::emit_enum_props, flo::emit_flo_props, flu::emit_flu_props,
    idk::emit_idk_props, param::emit_param_props,
};
use std::{collections::HashMap, fs, path::Path};

fn decode<T: CacheType>(data: &[u8]) -> T {
    let mut buf = Packet::from(data.to_vec());
    let mut value = T::new(0);
    value.decode(&mut buf);
    assert_eq!(buf.pos, data.len());
    value
}

#[test]
fn rendering_fields_keep_their_wire_values() {
    let loc: LocTypeRaw = decode(&[28, 255, 39, 128, 70, 128, 0, 71, 127, 255, 72, 255, 255, 0]);
    assert_eq!(loc.wallwidth, 255);
    assert_eq!(loc.contrast, -128);
    assert_eq!((loc.offsetx, loc.offsety, loc.offsetz), (-32768, 32767, -1));
    #[cfg(since_244)]
    {
        let npc: NpcTypeRaw = decode(&[101, 255, 0]);
        let obj: ObjTypeRaw = decode(&[114, 127, 0]);
        assert_eq!(npc.contrast, -1);
        assert_eq!(obj.contrast, 127);
    }
}

#[cfg(since_647)]
#[test]
fn xtea_json_preserves_signed_keys_and_rejects_invalid_records() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("xtea_json");
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("keys.json");
    fs::write(
        &path,
        r#"[
            {"archive":5,"name_hash":-123,"key":[-2147483648,-1,0,2147483647],"group":42},
            {"archive":7,"name_hash":456,"key":[1,2,3,4]}
        ]"#,
    )
    .unwrap();
    assert_eq!(
        load_xtea_keys(&path).unwrap(),
        HashMap::from([(-123, [i32::MIN, -1, 0, i32::MAX])])
    );
    for json in [
        "{}",
        "[null]",
        r#"[{"name_hash":1,"key":[1,2,3,4]}]"#,
        r#"[{"archive":5,"key":[1,2,3,4]}]"#,
        r#"[{"archive":5,"name_hash":1}]"#,
        r#"[{"archive":256,"name_hash":1,"key":[1,2,3,4]}]"#,
        r#"[{"archive":5,"name_hash":2147483648,"key":[1,2,3,4]}]"#,
        r#"[{"archive":5,"name_hash":1,"key":[1,2,3]}]"#,
        r#"[{"archive":5,"name_hash":1,"key":[1,2,3,4,5]}]"#,
        r#"[{"archive":5,"name_hash":1,"key":[2147483648,2,3,4]}]"#,
        r#"[{"archive":5,"name_hash":1,"key":["1",2,3,4]}]"#,
    ] {
        fs::write(&path, json).unwrap();
        assert!(load_xtea_keys(&path).is_err(), "accepted {json}");
    }
}

#[cfg(since_647)]
#[test]
fn wearable_ids_include_linked_items_and_both_genders() {
    let mut objs: Vec<_> = (0..7).map(ObjTypeRaw::new).collect();
    objs[1].manwear = Some(123);
    objs[2].womanwear = Some(456);
    objs[3].wearpos = Some(WearPos::Ring);
    objs[4].lenttemplate = 6;
    objs[4].lentlink = 1;
    objs[5].boughttemplate = 6;
    objs[5].boughtlink = 2;
    let objs = TypeProvider::<ObjType>::from_types(
        objs,
        ObjContext {
            members: true,
            autodisable_params: Box::new([]),
        },
    );
    assert_eq!(
        objs.types.iter().map(|obj| obj.wearid).collect::<Vec<_>>(),
        [None, Some(0), Some(1), None, Some(2), Some(3), None]
    );
}

#[cfg(since_647)]
#[test]
fn modern_script_types_survive_projection() {
    for (byte, expected) in [
        (64, ScriptVarType::Cursor),
        (96, ScriptVarType::MapArea),
        (100, ScriptVarType::Graphic),
        (106, ScriptVarType::Jingle),
        (107, ScriptVarType::ChatCat),
        (109, ScriptVarType::Model),
        (120, ScriptVarType::Texture),
        (128, ScriptVarType::Bas),
        (171, ScriptVarType::Vorbis),
        (181, ScriptVarType::MapElement),
    ] {
        let raw: EnumTypeRaw = decode(&[1, byte, 2, byte, 0]);
        let value = EnumType::from(raw);
        assert_eq!((value.inputtype, value.outputtype), (expected, expected));
        let raw: ParamTypeRaw = decode(&[1, byte, 0]);
        assert_eq!(ParamType::from(raw).var_type, expected);
    }
}

#[cfg(since_647)]
#[test]
fn explicit_defaults_and_sentinels_survive_unpacking() {
    let mut registry = UnpackedPacks::new();
    let empty: EnumTypeRaw = decode(&[0]);
    assert_eq!(
        (empty.inputtype, empty.outputtype),
        (ScriptVarType::Int, ScriptVarType::Int)
    );
    assert!(emit_enum_props(&empty, &mut registry, &HashMap::new()).is_empty());
    let explicit: EnumTypeRaw = decode(&[1, b'i', 2, b'i', 4, 0, 0, 0, 0, 0]);
    assert_eq!(
        emit_enum_props(&explicit, &mut registry, &HashMap::new()),
        [
            ("inputtype".into(), "int".into()),
            ("outputtype".into(), "int".into()),
            ("default".into(), "0".into()),
        ]
    );
    let empty: ParamTypeRaw = decode(&[0]);
    assert_eq!(empty.var_type, ScriptVarType::Int);
    assert!(emit_param_props(&empty, &mut registry, &HashMap::new()).is_empty());
    let explicit: ParamTypeRaw = decode(&[1, b'i', 2, 0, 0, 0, 0, 0]);
    assert_eq!(
        emit_param_props(&explicit, &mut registry, &HashMap::new()),
        [
            ("type".into(), "int".into()),
            ("default".into(), "0".into()),
        ]
    );
    let idk: IdkTypeRaw = decode(&[1, 0, 60, 0, 0, 62, 255, 255, 0]);
    let props = emit_idk_props(&idk, &mut registry, &HashMap::new());
    assert_eq!(
        props
            .iter()
            .map(|(key, _)| key.as_str())
            .collect::<Vec<_>>(),
        ["bodypart", "head1", "head3"]
    );
    let flo: FloTypeRaw = decode(&[1, 0, 0, 0, 2, 0, 3, 1, 0, 0]);
    assert_eq!(
        emit_flo_props(&flo, &mut registry, &HashMap::new()),
        [
            ("colour".into(), "0x0".into()),
            ("unknown2".into(), "0".into()),
            ("material".into(), "material_256".into()),
        ]
    );
    assert_eq!(FloType::from(flo).texture, Some(256));
    let flo: FloTypeRaw = decode(&[3, 255, 255, 0]);
    assert_eq!(
        emit_flo_props(&flo, &mut registry, &HashMap::new()),
        [("material".into(), "null".into())]
    );
    assert_eq!(FloType::from(flo).texture, None);
    let flu: FluType = decode(&[1, 0, 0, 0, 2, 255, 255, 3, 0, 0, 0]);
    assert_eq!(
        emit_flu_props(&flu, &mut registry, &HashMap::new()),
        [
            ("colour".into(), "0x0".into()),
            ("material".into(), "null".into()),
            ("texturescale".into(), "0".into()),
        ]
    );
}

#[cfg(since_647)]
#[test]
fn dbrow_packer_writes_the_schema_width() {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join("dbrow_width");
    fs::create_dir_all(&path).unwrap();
    fs::write(path.join("dbrow.pack"), "0=sparse\n1=empty\n").unwrap();
    fs::write(path.join("dbtable.pack"), "0=table\n").unwrap();
    fs::write(
        path.join("test.dbtable"),
        "[table]\ncolumn=a,int\ncolumn=b,int\ncolumn=c,int,LIST\ncolumn=d,int\n",
    )
    .unwrap();
    let source = path.join("test.dbrow");
    fs::write(
        &source,
        "[sparse]\ntable=table\ndata=c,7\ndata=c,9\n[empty]\ntable=table\n",
    )
    .unwrap();
    let registry = PackRegistry::load(&path).unwrap();
    let packed = pack_dbrows(&FileCache::new(&path), &registry, &HashMap::new()).unwrap();
    let rows = TypeProvider::<DbRowType>::from_bytes::<DbRowType>(&packed.server.dat, ());
    for row in &rows.types {
        assert_eq!(row.types.as_ref().unwrap().len(), 4);
        let columns = row.columns.as_ref().unwrap();
        assert_eq!(columns.len(), 4);
        assert!(columns[0].is_none() && columns[1].is_none() && columns[3].is_none());
    }
    let values = rows.types[0].columns.as_ref().unwrap()[2].as_ref().unwrap();
    assert!(matches!(
        values.as_ref(),
        [DbRowValue::Int(7), DbRowValue::Int(9)]
    ));
    assert!(rows.types[1].columns.as_ref().unwrap()[2].is_none());
    fs::write(&source, "[sparse]\ntable=table\ndata=unknown,7\n").unwrap();
    assert!(pack_dbrows(&FileCache::new(&path), &registry, &HashMap::new()).is_err());
}

#[test]
fn params_share_type_key_value_order() {
    #[cfg(before_647)]
    let terminator = 10;
    #[cfg(since_647)]
    let terminator = 0;
    let data = [
        249, 2, 0, 1, 35, 69, 255, 255, 255, 214, 1, 2, 52, 86, b'o', b'a', b'k', terminator, 249,
        1, 0, 1, 35, 69, 0, 0, 0, 7, 249, 0, 250, b't', b'e', b's', b't', terminator, 0,
    ];
    let loc: LocTypeRaw = decode(&data);
    let npc: NpcTypeRaw = decode(&data);
    let obj: ObjTypeRaw = decode(&data);
    let raw: StructTypeRaw = decode(&data);
    assert_eq!(loc.debugname(), Some("test"));
    assert_eq!(npc.debugname(), Some("test"));
    assert_eq!(obj.debugname(), Some("test"));
    assert_eq!(raw.debugname(), Some("test"));
    for codes in [&loc.opcode_order, &npc.opcode_order, &obj.opcode_order] {
        assert_eq!(codes.as_slice(), [249, 249, 249, 250]);
    }
    for records in [&loc.raw_opcodes, &npc.raw_opcodes, &obj.raw_opcodes] {
        assert_eq!(records[0].1.as_slice(), &data[1..18]);
        assert_eq!(records[1].1.as_slice(), &data[19..28]);
        assert_eq!(records[2].1.as_slice(), &[0]);
    }
    let entries = raw.params.as_ref().unwrap();
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].0, 0x12345);
    assert!(matches!(entries[0].1, ParamValue::Int(-42)));
    let value = StructType::from(raw);
    for params in [loc.params, npc.params, obj.params, value.params] {
        let params = params.unwrap();
        assert_eq!(params.len(), 2);
        assert!(matches!(params[&0x12345], ParamValue::Int(7)));
        assert!(matches!(&params[&0x23456], ParamValue::String(value) if value.as_ref() == "oak"));
    }
}

#[cfg(before_647)]
#[test]
fn server_param_packer_writes_type_before_key() {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join("param_layout");
    fs::create_dir_all(&path).unwrap();
    fs::write(
        path.join("param.pack"),
        "4660=int_param\n9029=string_param\n",
    )
    .unwrap();
    let registry = PackRegistry::load(&path).unwrap();
    let types = HashMap::from([
        ("int_param".into(), "int".into()),
        ("string_param".into(), "string".into()),
    ]);
    let props = [
        ("param".into(), "int_param,-42".into()),
        ("param".into(), "string_param,oak".into()),
    ];
    let mut packed = PackedData::new(1);
    packed.start_entry();
    parse_params(&registry, &types, &mut packed, &props, "test");
    packed.finish_entry();
    assert_eq!(
        packed.dat,
        [
            0, 1, 249, 2, 0, 0, 18, 52, 255, 255, 255, 214, 1, 0, 35, 69, b'o', b'a', b'k', 10, 0
        ]
    );
    let value: StructTypeRaw = decode(&packed.dat[2..]);
    let entries = value.params.unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].0, 4660);
    assert!(matches!(entries[0].1, ParamValue::Int(-42)));
    assert_eq!(entries[1].0, 9029);
    assert!(matches!(&entries[1].1, ParamValue::String(value) if value.as_ref() == "oak"));
}

#[test]
fn loc_models_preserve_shapes_and_groups() {
    #[cfg(before_647)]
    let data = [1, 2, 1, 35, 10, 4, 86, 4, 14, 3, 0];
    #[cfg(since_647)]
    let data = [1, 2, 10, 2, 1, 35, 4, 86, 255, 0, 14, 3, 0];
    let loc: LocTypeRaw = decode(&data);
    let models = loc.models.as_ref().unwrap();
    assert_eq!(models.len(), 2);
    assert_eq!(models[0].shape, 10);
    #[cfg(before_647)]
    {
        assert_eq!(models[0].model, 291);
        assert_eq!(models[1].model, 1110);
        assert_eq!(models[1].shape, 4);
    }
    #[cfg(since_647)]
    {
        assert_eq!(models[0].models.as_ref(), [291, 1110]);
        assert!(models[1].models.is_empty());
        assert_eq!(models[1].shape as i8, -1);
    }
    assert_eq!(loc.width, 3);
    assert_eq!(loc.opcode_order, [1, 14]);
    let mut locs = vec![loc];
    LocTypeRaw::post_decode(&mut locs, &());
    assert_eq!(locs[0].active, Some(u8::from(cfg!(before_647))));
    let empty: LocTypeRaw = decode(&[1, 0, 14, 7, 0]);
    assert!(empty.models.unwrap().is_empty());
    assert_eq!(empty.width, 7);
}

#[cfg(since_254)]
#[test]
fn loc_opcode_five_preserves_each_model_list() {
    #[cfg(before_647)]
    let data = [5, 2, 1, 35, 4, 86, 14, 3, 0];
    #[cfg(since_647)]
    let data = [5, 1, 10, 2, 1, 35, 4, 86, 2, 4, 0, 10, 1, 7, 137, 14, 3, 0];
    let loc: LocTypeRaw = decode(&data);
    let models = loc.models.unwrap();
    #[cfg(before_647)]
    {
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].model, 291);
        assert_eq!(models[1].model, 1110);
        assert!(models.iter().all(|model| model.shape == 10));
    }
    #[cfg(since_647)]
    {
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].models.as_ref(), [291, 1110]);
        let hd = loc.hd_models.unwrap();
        assert_eq!(hd.len(), 2);
        assert_eq!(hd[0].shape, 4);
        assert!(hd[0].models.is_empty());
        assert_eq!(hd[1].shape, 10);
        assert_eq!(hd[1].models.as_ref(), [1929]);
    }
    assert_eq!(loc.width, 3);
}

#[test]
fn recolours_preserve_pair_order() {
    let data = [40, 2, 1, 2, 3, 4, 255, 255, 18, 52, 0];
    let loc: LocTypeRaw = decode(&data);
    let npc: NpcTypeRaw = decode(&data);
    let obj: ObjTypeRaw = decode(&data);
    for (src, dst) in [
        (loc.recol_s, loc.recol_d),
        (npc.recol_s, npc.recol_d),
        (obj.recol_s, obj.recol_d),
    ] {
        assert_eq!(src.unwrap().as_ref(), [258, 65535]);
        assert_eq!(dst.unwrap().as_ref(), [772, 4660]);
    }
}

#[cfg(since_647)]
#[test]
fn retextures_preserve_each_clients_pair_order() {
    let data = [41, 1, 0, 7, 0, 9, 0];
    let loc: LocTypeRaw = decode(&data);
    let npc: NpcTypeRaw = decode(&data);
    let idk: IdkTypeRaw = decode(&data);
    let obj: ObjTypeRaw = decode(&data);
    let spot: SpotAnimTypeRaw = decode(&data);
    for (src, dst, expected) in [
        (loc.retex_s, loc.retex_d, (7, 9)),
        (npc.retex_s, npc.retex_d, (7, 9)),
        (idk.retex_s, idk.retex_d, (7, 9)),
        (obj.retex_s, obj.retex_d, (9, 7)),
        (spot.retex_s, spot.retex_d, (9, 7)),
    ] {
        assert_eq!(src.unwrap().as_ref(), [expected.0]);
        assert_eq!(dst.unwrap().as_ref(), [expected.1]);
    }
}

#[test]
fn sequence_frames_preserve_interleaved_and_split_layouts() {
    #[cfg(before_647)]
    let data = [1, 2, 18, 52, 255, 255, 0, 7, 171, 205, 0, 9, 0, 11, 5, 8, 0];
    #[cfg(since_647)]
    let data = [1, 0, 2, 0, 7, 0, 11, 18, 52, 171, 205, 0, 1, 0, 2, 5, 8, 0];
    let seq: SeqTypeRaw = decode(&data);
    assert_eq!(seq.delays.unwrap().as_ref(), [7, 11]);
    assert_eq!(seq.priority, 8);
    #[cfg(before_647)]
    {
        assert_eq!(seq.frames.unwrap().as_ref(), [0x1234, 0xabcd]);
        assert_eq!(seq.iframes.unwrap().as_ref(), [None, Some(9)]);
    }
    #[cfg(since_647)]
    {
        assert_eq!(seq.frames.unwrap().as_ref(), [0x11234, 0x2abcd]);
        assert!(seq.iframes.is_none());
        assert_eq!(seq.duration, 18);
    }
}

#[cfg(since_289)]
#[test]
fn transforms_preserve_missing_ids_and_default_targets() {
    #[cfg(before_647)]
    let data = [77, 0, 5, 1, 255, 254, 255, 255, 14, 3, 0];
    #[cfg(since_647)]
    let data = [92, 0, 5, 255, 255, 0, 99, 1, 255, 254, 255, 255, 14, 3, 0];
    let loc: LocTypeRaw = decode(&data);
    assert_eq!(loc.width, 3);
    #[cfg(before_647)]
    {
        assert_eq!(loc.multivar, Some(5));
        assert_eq!(loc.multiloc.unwrap().as_ref(), [65534, -1]);
    }
    #[cfg(since_647)]
    {
        assert_eq!((loc.transform_varbit, loc.transform_varp), (5, -1));
        assert_eq!(loc.multiloc.unwrap().as_ref(), [65534, -1, 99]);
        let npc: NpcTypeRaw =
            decode(&[118, 0, 5, 255, 255, 0, 99, 1, 255, 254, 255, 255, 12, 3, 0]);
        assert_eq!((npc.transform_varbit, npc.transform_varp), (5, -1));
        assert_eq!(npc.transforms.unwrap().as_ref(), [65534, -1, 99]);
        assert_eq!(npc.size, 3);
    }
}

#[test]
fn inventory_server_codes_use_upstream_layout() {
    let inv: InvTypeRaw = decode(&[
        4, 1, 0, 1, 0, 2, 0, 0, 0, 3, 4, 2, 0, 4, 0, 5, 0, 0, 0, 6, 255, 255, 0, 7, 255, 255, 255,
        255, 2, 0, 9, 3, 5, 6, 7, 8, 9, 0,
    ]);
    assert_eq!(inv.size, 9);
    assert_eq!(inv.stockobj.unwrap().as_ref(), [4, u16::MAX]);
    assert_eq!(inv.stockcount.unwrap().as_ref(), [5, 7]);
    assert_eq!(inv.stockrate.unwrap().as_ref(), [6, -1]);
    assert!(inv.stackall && inv.restock && inv.allstock && inv.runweight && inv.dummyinv);
    assert!(!inv.protect);
    let empty: InvTypeRaw = decode(&[4, 1, 0, 1, 0, 2, 0, 0, 0, 3, 4, 0, 0]);
    assert!(empty.stockobj.unwrap().is_empty());
    assert!(empty.stockcount.unwrap().is_empty());
    assert!(empty.stockrate.unwrap().is_empty());
}

#[test]
fn npc_hunt_ids_survive_packing() {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join("npc_hunt_width");
    fs::create_dir_all(&path).unwrap();
    fs::write(path.join("npc.pack"), "0=hellhound\n").unwrap();
    fs::write(path.join("hunt.pack"), "255=cowardly\n").unwrap();
    fs::write(
        path.join("test.npc"),
        "[hellhound]\nname=Hellhound\nhuntmode=cowardly\ntimer=257\n",
    )
    .unwrap();
    let registry = PackRegistry::load(&path).unwrap();
    let (server, client) = {
        let packed = pack_npcs(
            &FileCache::new(&path),
            &registry,
            &HashMap::new(),
            &HashMap::new(),
        )
        .unwrap();
        (packed.server.dat, packed.client.unwrap().dat)
    };
    let npc: NpcTypeRaw = decode(&server[2..]);
    assert_eq!(npc.huntmode, Some(255));
    assert_eq!(npc.timer, Some(257));
    assert_eq!(npc.debugname(), Some("hellhound"));
    let npc: NpcTypeRaw = decode(&client[2..]);
    assert_eq!(npc.huntmode, None);
    assert_eq!(npc.name.as_deref(), Some("Hellhound"));
    #[cfg(since_647)]
    {
        fs::write(path.join("hunt.pack"), "396=cowardly\n").unwrap();
        let registry = PackRegistry::load(&path).unwrap();
        let packed = pack_npcs(
            &FileCache::new(&path),
            &registry,
            &HashMap::new(),
            &HashMap::new(),
        )
        .unwrap();
        let npc: NpcTypeRaw = decode(&packed.server.dat[2..]);
        assert_eq!(npc.huntmode, Some(396));

        fs::write(path.join("hunt.pack"), "65536=cowardly\n").unwrap();
        let registry = PackRegistry::load(&path).unwrap();
        let error = pack_npcs(
            &FileCache::new(&path),
            &registry,
            &HashMap::new(),
            &HashMap::new(),
        )
        .err()
        .expect("Oversized hunt ID must fail packing");
        assert!(
            error
                .to_string()
                .contains("huntmode 'cowardly' exceeds 65535")
        );
    }
}

#[cfg(since_647)]
#[test]
fn bas_anims_keep_their_names_weights_and_order() {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join("bas_round_trip");
    fs::create_dir_all(&path).unwrap();
    fs::write(
        path.join("seq.pack"),
        "737=human_climbing\n738=human_climbing_ready\n",
    )
    .unwrap();
    fs::write(path.join("bas.pack"), "0=bas_0\n").unwrap();
    let data = [
        38, 2, 225, 6, 2, 225, 1, 2, 226, 255, 255, 52, 2, 2, 225, 7, 234, 96, 3, 30, 2, 225, 55,
        2, 2, 225, 0,
    ];
    let mut packs = UnpackedPacks::new();
    packs.load_existing_names(&path);
    let text: String = emit_bas_props(&decode::<BasType>(&data), &mut packs, &HashMap::new())
        .iter()
        .map(|(key, value)| format!("{key}={value}\n"))
        .collect();
    assert_eq!(
        text,
        "readyanim_l=human_climbing\nrunanim=human_climbing\n\
         readyanim=human_climbing_ready,null\n\
         randomreadyanim=human_climbing,7\nrandomreadyanim=seq_60000,3\n\
         turnmax=737\nunknown55=2,737\n"
    );
    fs::write(path.join("all.bas"), format!("[bas_0]\n{text}")).unwrap();
    let registry = PackRegistry::load(&path).unwrap();
    let packed = pack_bases(&FileCache::new(&path), &registry, &HashMap::new()).unwrap();
    assert_eq!(packed.client.unwrap().dat[2..], data);
}

#[cfg(since_647)]
#[test]
fn particles_keep_their_keys_in_cache_order_and_repeats() {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join("particles");
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(path.join("_unpack")).unwrap();
    fs::write(path.join("particleemitter.pack"), "0=pem_0\n").unwrap();
    fs::write(path.join("particleeffector.pack"), "0=pef_0\n1=pef_1\n").unwrap();
    fs::write(
        path.join("_unpack/all.particleemitter"),
        "// 0\n[pem_0]\nspeedcontrol=2,-6\nlinkcolourchannels=no\nlocaleffectors=1,0\n\
         unknown16=1,32767,32767,1\nmaterial=816\nlinkcolourchannels=no\nrgba=-13421824,1\n",
    )
    .unwrap();
    fs::write(
        path.join("_unpack/all.particleeffector"),
        "// 0\n[pef_0]\nfalloff=1,409\nforcedirection=98,0,-42\nunknown9=yes\nforcetype=2\n",
    )
    .unwrap();
    let registry = PackRegistry::load(&path).unwrap();
    let cache = FileCache::new(&path);
    let first = |client: PackedData| {
        PackedData::entries(&client.dat, &client.idx)
            .unwrap()
            .next()
            .unwrap()
            .1
            .to_vec()
    };
    let emitter = first(
        pack_particleemitters(&cache, &registry, &HashMap::new())
            .unwrap()
            .client
            .unwrap(),
    );
    assert_eq!(
        emitter,
        [
            4, 2, 0xfa, 24, 10, 2, 0, 1, 0, 0, 16, 1, 0x7f, 0xff, 0x7f, 0xff, 1, 15, 3, 0x30, 24,
            6, 0xff, 0x33, 0x33, 0x00, 0, 0, 0, 1, 0
        ]
    );
    let mut def = ParticleEmitterType::new(0);
    def.decode(&mut Packet::from(emitter));
    let mut packs = UnpackedPacks::new();
    packs.load_existing_names(&path);
    let props = emit_particleemitter_props(&def, &mut packs, &HashMap::new());
    let expected = [
        ("speedcontrol", "2,-6"),
        ("linkcolourchannels", "no"),
        ("localeffectors", "1,0"),
        ("unknown16", "1,32767,32767,1"),
        ("material", "816"),
        ("linkcolourchannels", "no"),
        ("rgba", "-13421824,1"),
    ]
    .map(|(key, value)| (key.to_owned(), value.to_owned()));
    assert_eq!(props, expected);

    let effector = first(
        pack_particleeffectors(&cache, &registry, &HashMap::new())
            .unwrap()
            .client
            .unwrap(),
    );
    assert_eq!(
        effector,
        [
            4, 1, 0, 0, 1, 0x99, 3, 0, 0, 0, 98, 0, 0, 0, 0, 0xff, 0xff, 0xff, 0xd6, 9, 6, 2, 0
        ]
    );
    let mut def = ParticleEffectorType::new(0);
    def.decode(&mut Packet::from(effector));
    let props = emit_particleeffector_props(&def, &mut packs, &HashMap::new());
    let expected = [
        ("falloff", "1,409"),
        ("forcedirection", "98,0,-42"),
        ("unknown9", "yes"),
        ("forcetype", "2"),
    ]
    .map(|(key, value)| (key.to_owned(), value.to_owned()));
    assert_eq!(props, expected);
}

#[cfg(since_647)]
#[test]
fn billboards_keep_their_keys_in_cache_order() {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join("billboards");
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(path.join("_unpack")).unwrap();
    fs::write(
        path.join("billboard.pack"),
        "0=billboard_0\n2=billboard_2\n",
    )
    .unwrap();
    fs::write(
        path.join("_unpack/all.billboard"),
        "// 0\n[billboard_0]\nsize=50,51\nunknown3=0\nsourceblendmode=1\nunknown7=yes\n\
         destinationblendmode=2\nunknown6=yes\nmaterial=material_735\n\n// 2\n[billboard_2]\nmaterial=3\n",
    )
    .unwrap();
    let registry = PackRegistry::load(&path).unwrap();
    let client = pack_billboards(&FileCache::new(&path), &registry, &HashMap::new())
        .unwrap()
        .client
        .unwrap();
    let entries: Vec<_> = PackedData::entries(&client.dat, &client.idx)
        .unwrap()
        .map(|(_, data)| data.to_vec())
        .collect();
    let first = vec![2, 0, 50, 0, 51, 3, 0, 4, 1, 7, 5, 2, 6, 1, 2, 0xdf, 0];
    assert_eq!(entries, [first.clone(), Vec::new(), vec![1, 0, 3, 0]]);

    let mut def = BillboardType::new(0);
    def.decode(&mut Packet::from(first));
    let props = emit_billboard_props(&def, &mut UnpackedPacks::new(), &HashMap::new());
    let expected = [
        ("size", "50,51"),
        ("unknown3", "0"),
        ("sourceblendmode", "1"),
        ("unknown7", "yes"),
        ("destinationblendmode", "2"),
        ("unknown6", "yes"),
        ("material", "material_735"),
    ]
    .map(|(key, value)| (key.to_owned(), value.to_owned()));
    assert_eq!(props, expected);
}

#[cfg(since_647)]
#[test]
fn empty_tables_come_from_their_files() {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join("empty_tables");
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    fs::write(path.join("all.area"), "// 0\n[area_0]\n\n// 2\n[area_2]\n").unwrap();
    fs::write(path.join("all.varcon"), "[config43_0]\nfoo=1\n").unwrap();
    let area = pack_empty_table(&path, "area").unwrap();
    let entries: Vec<_> = PackedData::entries(&area.dat, &area.idx)
        .unwrap()
        .map(|(id, data)| (id, data.to_vec()))
        .collect();
    assert_eq!(entries, [(0, vec![0]), (1, vec![]), (2, vec![0])]);
    assert_eq!(pack_empty_table(&path, "varobj").unwrap().dat, [0, 0]);
    assert!(pack_empty_table(&path, "varcon").is_err());
}

#[cfg(since_647)]
#[test]
fn client_var_tables_write_their_opcodes() {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join("client_var_tables");
    fs::create_dir_all(path.join("_unpack")).unwrap();
    fs::write(path.join("varc.pack"), "0=varc_0\n2=named\n").unwrap();
    fs::write(path.join("varcs.pack"), "0=varc_3\n1=varc_4\n").unwrap();
    fs::write(
        path.join("_unpack/all.varc"),
        "// 0\n[varc_0]\ntype=int\nscope=perm\n\n// 2\n[named]\ntype=namedobj\n\n\
         // 0\n[varc_3]\ntype=string\n\n// 1\n[varc_4]\ntype=string\nscope=perm\n",
    )
    .unwrap();
    fs::write(path.join("varclansetting.pack"), "0=a\n1=b\n").unwrap();
    fs::write(
        path.join("_unpack/all.varclansetting"),
        "[a]\ntype=long\n\n[b]\ntype=byte:0x01\nbasevar=3\nstartbit=4\nendbit=5\n",
    )
    .unwrap();
    let registry = PackRegistry::load(&path).unwrap();
    let cache = FileCache::new(&path);
    let constants = HashMap::new();
    let bytes = |client: PackedData| -> Vec<Vec<u8>> {
        PackedData::entries(&client.dat, &client.idx)
            .unwrap()
            .map(|(_, data)| data.to_vec())
            .collect()
    };
    let varcs = pack_varcs(&cache, &registry.varc, &constants).unwrap();
    assert_eq!(
        bytes(varcs.client.unwrap()),
        [vec![1, 105, 2, 0], Vec::new(), vec![1, b'O', 0]]
    );
    let strings = pack_varc_strings(&cache, &registry.varcs, &constants).unwrap();
    assert_eq!(bytes(strings.client.unwrap()), [vec![0], vec![2, 0]]);
    let settings = pack_varclansettings(&cache, &registry.varclansetting, &constants).unwrap();
    assert_eq!(
        bytes(settings.client.unwrap()),
        [vec![1, 0xa7, 0], vec![1, 1, 2, 0, 3, 4, 5, 0]]
    );
}

#[cfg(since_647)]
#[test]
fn server_only_table_keeps_the_cache_ids() {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join("server_only_table");
    fs::create_dir_all(path.join("_unpack")).unwrap();
    fs::write(path.join("hunt.pack"), "0=first\n2=third\n3=ours\n").unwrap();
    fs::write(
        path.join("_unpack/all.hunt"),
        "[first]\ntype=player\n\n[third]\n",
    )
    .unwrap();
    fs::write(path.join("ours.hunt"), "[ours]\ntype=npc\n").unwrap();
    let registry = PackRegistry::load(&path).unwrap();
    let packed = pack_hunts(&FileCache::new(&path), &registry, &HashMap::new()).unwrap();
    let client = packed.client.unwrap();
    let entries: Vec<_> = PackedData::entries(&client.dat, &client.idx)
        .unwrap()
        .map(|(id, data)| (id, data.to_vec()))
        .collect();
    assert_eq!(
        entries,
        [(0, vec![0]), (1, Vec::new()), (2, vec![0])],
        "every table id is empty, a gap stays a gap and ours stays past the table"
    );
    let server: Vec<_> = PackedData::entries(&packed.server.dat, &packed.server.idx)
        .unwrap()
        .map(|(_, data)| decode::<HuntType>(data))
        .collect();
    assert_eq!(server.len(), 4);
    assert_eq!(server[0].hunt_type, HuntModeType::Player);
    assert_eq!(server[3].hunt_type, HuntModeType::Npc);
}

#[test]
fn npc_server_codes_use_upstream_layout() {
    let npc: NpcTypeRaw = decode(&[
        19,
        1,
        2,
        20,
        2,
        3,
        21,
        3,
        4,
        22,
        6,
        23,
        1,
        24,
        2,
        25,
        #[cfg(since_647)]
        0,
        255,
        26,
        4,
        5,
        27,
        5,
        6,
        28,
        7,
        29,
        255,
        35,
        1,
        1,
        2,
        3,
        4,
        8,
        36,
        37,
        0,
    ]);
    assert_eq!(npc.timer, Some(0x102));
    assert_eq!(npc.respawnrate, 0x203);
    assert_eq!(npc.regenrate, 0x304);
    assert_eq!(npc.moverestrict as u8, 6);
    assert_eq!(npc.blockwalk as u8, 1);
    assert_eq!(npc.defaultmode as u8, 2);
    assert_eq!(npc.huntmode, Some(255));
    assert_eq!(npc.wanderrange, 0x405);
    assert_eq!(npc.maxrange, Some(0x506));
    assert_eq!(npc.huntrange, 7);
    assert_eq!(npc.attackrange, 255);
    let patrol = npc.patrol.unwrap();
    assert_eq!(patrol.len(), 1);
    assert_eq!((patrol[0].coord, patrol[0].delay), (0x1020304, 8));
    assert!(npc.members);
    assert!(!npc.givechase);
}

#[test]
fn hunt_afk_packing_preserves_explicit_and_default_values() {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join("hunt_afk");
    fs::create_dir_all(&path).unwrap();
    fs::write(path.join("hunt.pack"), "0=test\n").unwrap();
    for (property, expected) in [
        ("check_afk=off", HuntCheckAfk::Off),
        ("check_afk=on", HuntCheckAfk::On),
        ("", HuntCheckAfk::On),
    ] {
        fs::write(path.join("test.hunt"), format!("[test]\n{property}\n")).unwrap();
        let packed = pack_hunts(
            &FileCache::new(&path),
            &PackRegistry::load(&path).unwrap(),
            &HashMap::new(),
        )
        .unwrap();
        let hunt: HuntType = decode(&packed.server.dat[2..]);
        assert_eq!(hunt.check_afk, expected);
    }
}

#[test]
fn hunt_server_codes_use_upstream_layout() {
    #[cfg(before_647)]
    let terminator = 10;
    #[cfg(since_647)]
    let terminator = 0;
    let hunt: HuntType = decode(&[
        16, 0, 1, 0, 2, b'>', terminator, 0, 0, 0, 3, 17, 0, 4, 0, 5, b'<', terminator, 0, 0, 0, 6,
        18, 0, 7, 0, 8, b'=', terminator, 0, 0, 0, 9, 19, 0, 10, b'!', terminator, 0, 0, 0, 11, 20,
        0, 12, b'&', terminator, 0, 0, 0, 13, 21, 0, 14, b'|', terminator, 255, 255, 255, 255, 0,
    ]);
    let inv = hunt.check_inv.unwrap();
    assert_eq!(
        (inv.inv, inv.obj, inv.condition.as_str(), inv.value),
        (1, 2, ">", 3)
    );
    let param = hunt.check_invparam.unwrap();
    assert_eq!(
        (
            param.inv,
            param.param,
            param.condition.as_str(),
            param.value
        ),
        (4, 5, "<", 6)
    );
    let category = hunt.check_invcat.unwrap();
    assert_eq!(
        (
            category.inv,
            category.category,
            category.condition.as_str(),
            category.value
        ),
        (7, 8, "=", 9)
    );
    assert_eq!(hunt.extracheck_vars.len(), 3);
    for (check, expected) in
        hunt.extracheck_vars
            .iter()
            .zip([(10, "!", 11), (12, "&", 13), (14, "|", -1)])
    {
        assert_eq!(
            (check.varp, check.condition.as_str(), check.value),
            expected
        );
    }
}
