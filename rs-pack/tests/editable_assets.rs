#![cfg(rev = "647")]

use rs_pack::archive;
use rs_pack::pack::{
    media,
    model::source,
    names::AssetNames,
    other::{asset, material, shader, worldmap_source},
    sound::{vorbis, vorbis_setup},
};
use rs_pack::unpack;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

fn content() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("content/647")
}

fn names() -> AssetNames {
    AssetNames::load(&content().join("pack")).unwrap()
}

#[test]
fn worldmap_squares_preserve_chunk_order_and_edits() {
    let names = names();
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("editable-worldmap-{}", std::process::id()));
    let path = dir.join("main.fm");
    let aid = archive::WORLDMAP;
    let mut data = vec![0, 0];
    for (x, chunk_x) in [(50, 0), (50, 1), (51, 0), (50, 2)] {
        data.extend_from_slice(&[1, x, 51, chunk_x, 0]);
        data.extend_from_slice(&[248; 64]);
    }
    let text = asset::unpack(aid, 0, &data, &names).unwrap();
    let source = unpack::asset::write_worldmap_squares(&path, 1, &text).unwrap();
    assert_eq!(
        source,
        "format=area\nunderlays=0\noverlays=0\n\
        square=main/file_1/m50_51.fms\n\
        square=main/file_1/m51_51.fms\n\
        square=main/file_1/m50_51_2.fms\n"
    );
    fs::write(&path, format!("[file_1]\n{source}")).unwrap();
    assert_eq!(
        asset::pack_file(aid, 0, &path, &names).unwrap(),
        [(1, data.clone())]
    );
    let source = worldmap_source::read_squares(&path, &fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(asset::pack_group(aid, 0, &source, &names).unwrap(), data);

    let square = dir.join("main/file_1/m50_51.fms");
    let text = fs::read_to_string(&square)
        .unwrap()
        .replacen("=empty\n", "=empty_overlay\n", 1);
    fs::write(square, text).unwrap();
    data[7] = 250;
    assert_eq!(
        asset::pack_file(aid, 0, &path, &names).unwrap(),
        [(1, data.clone())]
    );

    let manifest = fs::read_to_string(&path).unwrap();
    let inline = fs::read_to_string(dir.join("main/file_1/m51_51.fms")).unwrap();
    fs::write(
        &path,
        manifest.replace("square=main/file_1/m51_51.fms\n", &inline),
    )
    .unwrap();
    assert_eq!(
        asset::pack_file(aid, 0, &path, &names).unwrap(),
        [(1, data.clone())]
    );
    fs::write(&path, manifest).unwrap();

    fs::remove_file(dir.join("main/file_1/m51_51.fms")).unwrap();
    assert!(
        asset::pack_file(aid, 0, &path, &names)
            .unwrap_err()
            .to_string()
            .contains("m51_51.fms")
    );
    assert!(worldmap_source::read_squares(&path, "square=../outside.fms\n").is_err());
    fs::write(
        &path,
        "[file_1]\nformat=area\nunderlays=0\noverlays=0\nsquare=../outside.fms\n",
    )
    .unwrap();
    assert!(asset::pack_file(aid, 0, &path, &names).is_err());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn model_coordinates_derive_delta_widths_and_reject_missing_vertices() {
    let names = names();
    let text = "format=old\nvertices=3\nfaces=1\nmappings=0\nflags=0\npriority=0\nhas_alpha=0\nhas_face_labels=0\nhas_vertex_labels=0\n\
        vertex0=0,0,0\nvertex1=10,0,0\nvertex2=10,10,0\nface0.vertices=0,1,2\nface0.colour=500\n";
    let expected = [
        0, 1, 2, 1, 64, 65, 65, 1, 244, 74, 74, 0, 3, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 1, 0, 0, 0,
        3,
    ];
    assert_eq!(source::pack(text, &names).unwrap(), expected);
    let edited = source::pack(&text.replace("vertex2=10,10,0", "vertex2=10,80,0"), &names).unwrap();
    assert_eq!(
        &edited[..12],
        &[0, 1, 2, 1, 64, 65, 65, 1, 244, 74, 192, 80]
    );
    assert_eq!(&edited[edited.len() - 6..edited.len() - 4], &[0, 2]);
    assert!(
        source::unpack(&edited, &names)
            .unwrap()
            .contains("vertex2=10,80,0")
    );
    assert!(
        source::pack(
            &text.replace("face0.vertices=0,1,2", "face0.vertices=0,1,3"),
            &names
        )
        .is_err()
    );
}

#[test]
fn materials_pack_from_a_file_each_named_by_texture() {
    let names = names();
    let text = "highdetail=1\nlowmem=0\nlowdetail=0\nbrightness=0\nintensity=-1\neffect=0\n\
        effect_param=0\naveragecolour=6547\nscroll_u=0\nscroll_v=0\ncombinealpha=0\nflag95=0\n\
        mipmapping=2\nrepeat_s=1\nrepeat_t=1\nhdr=0\neffect_param2=0\neffect_param3=0\nalphamode=0\n";
    let materials = BTreeMap::from([
        (0, text.to_owned()),
        (2, text.replace("intensity=-1", "intensity=5")),
    ]);
    let data = material::pack(&materials, &names).unwrap();
    assert_eq!(data.len(), 5 + 2 * 23);
    assert_eq!(&data[..5], &[0, 3, 1, 0, 1]);
    assert_eq!(&data[13..15], &[255, 5]);
    assert_eq!(material::unpack(&data, &names).unwrap(), materials);

    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("editable-materials-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let paths: Vec<_> = materials
        .iter()
        .map(|(id, text)| {
            let path = dir.join(format!("{}.material", names.name("texture", *id)));
            fs::write(&path, text).unwrap();
            path
        })
        .collect();
    assert_eq!(material::pack_files(&paths, &names).unwrap(), data);
    let stray = dir.join("material_0.material");
    fs::write(&stray, text).unwrap();
    assert!(material::pack_files(&[stray], &names).is_err());
    let mut missing = materials.clone();
    missing.insert(1, "highdetail=1\n".to_owned());
    assert!(material::pack(&missing, &names).is_err());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn sprite_names_with_dots_preserve_palette_and_alpha() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("editable-sprite-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let data = [
        2, 1, 2, 128, 255, 0, 0, 0, 18, 52, 86, 0, 2, 0, 1, 2, 0, 0, 0, 0, 0, 2, 0, 1, 0, 1,
    ];
    unpack::media::write_sprite_group(&data, &dir.join("painting.02")).unwrap();
    assert!(dir.join("painting.02.tga").is_file());
    assert!(dir.join("painting.02.alpha.tga").is_file());
    assert!(!dir.join("painting.tga").exists());
    assert_eq!(
        media::encode_sprite_sheet(&dir.join("painting.02.sprite")).unwrap(),
        data
    );
    for name in [
        "painting.02.sprite",
        "painting.02.tga",
        "painting.02.alpha.tga",
    ] {
        fs::remove_file(dir.join(name)).unwrap();
    }
    fs::remove_dir(dir).unwrap();
}

#[test]
fn shader_edits_produce_real_instruction_tokens() {
    let bytes = shader::pack("version=vs_2_0\nmov oPos, v0\n").unwrap();
    let expected: Vec<u8> = [
        0xfffe0200u32,
        0x02000001,
        0xc00f0000,
        0x90e40000,
        0x0000ffff,
    ]
    .into_iter()
    .flat_map(u32::to_le_bytes)
    .collect();
    assert_eq!(bytes, expected);
    let bytes = shader::pack("version=vs_2_0\nmov oPos, -v1.x\n").unwrap();
    assert_eq!(
        u32::from_le_bytes(bytes[12..16].try_into().unwrap()),
        0x91000001
    );
    assert!(shader::pack("version=vs_2_0\nmov oPos, invalid\n").is_err());
}

#[test]
fn vorbis_metadata_is_editable_and_incompatible_setup_is_rejected() {
    let names = names();
    let dir = content().join("vorbis");
    let text = fs::read_to_string(dir.join(format!("{}.vorbis", names.name("vorbis", 0)))).unwrap();
    let mut setup = vorbis_setup::pack(&text, &names).unwrap();
    let audio = fs::read(dir.join(format!("{}.ogg", names.name("vorbis", 1)))).unwrap();
    let metadata =
        "sample_rate=22050\nsample_count=62110\nloop_start=123\nloop_end=456\nping_pong=yes\n";
    let shared = vorbis::Setup::new(&setup, &names).unwrap();
    let bytes = vorbis::pack_sample(metadata, &audio, &shared, &names).unwrap();
    assert_eq!(i32::from_be_bytes(bytes[8..12].try_into().unwrap()), 123);
    assert_eq!(i32::from_be_bytes(bytes[12..16].try_into().unwrap()), !456);
    setup[0] = 0xbb;
    let incompatible = vorbis::Setup::new(&setup, &names).unwrap();
    let error = vorbis::pack_sample(metadata, &audio, &incompatible, &names).unwrap_err();
    assert!(error.to_string().contains("shared Vorbis setup"));
}
