use crate::archive::{self, JagArchive};
use crate::types::OndemandBlobs;
use crate::versionlist::VersionListMeta;
use rs_io::Packet;
use rs_io::crc::getcrc;
use rs_io::jag::JagFile;

pub fn build_version_list(
    cache: &OndemandBlobs,
    meta: &VersionListMeta,
    midi_jingles: &[bool],
) -> Vec<u8> {
    let build = |index: u8, versions: &[u16], crcs: &[i32]| -> (Vec<u8>, Vec<u8>) {
        let n = versions.len();
        let mut ver = Packet::new(n * 2 + 16);
        let mut crc = Packet::new(n * 4 + 16);
        for (id, &v) in versions.iter().enumerate() {
            ver.p2(v);
            match cache[index as usize - 1].get(id).filter(|d| !d.is_empty()) {
                Some(blob) => crc.p4(getcrc(blob, 0, blob.len().saturating_sub(2))),
                None => crc.p4(crcs.get(id).copied().unwrap_or(0)),
            }
        }
        (finish(ver), finish(crc))
    };

    let (model_version, model_crc) = build(archive::MODELS, &meta.model_version, &meta.model_crc);
    let (anim_version, anim_crc) = build(archive::ANIMS, &meta.anim_version, &meta.anim_crc);
    let (midi_version, midi_crc) = build(archive::MIDI, &meta.midi_version, &meta.midi_crc);
    let (map_version, map_crc) = build(archive::MAPS, &meta.map_version, &meta.map_crc);

    let mut model_index = Packet::new(meta.model_flags.len() + 16);
    for &flag in &meta.model_flags {
        model_index.p1(flag);
    }
    let mut midi_index = Packet::new(meta.midi_version.len() + 16);
    for id in 0..meta.midi_version.len() {
        midi_index.p1(midi_jingles.get(id).copied().unwrap_or(false) as u8);
    }
    let mut map_index = Packet::new(meta.maps.len() * 7 + 16);
    for m in &meta.maps {
        map_index.p2(m.mapsquare);
        map_index.p2(m.land_file);
        map_index.p2(m.loc_file);
        map_index.p1(m.free2play as u8);
    }

    let tables = vec![
        ("model_version", model_version),
        ("model_crc", model_crc),
        ("model_index", finish(model_index)),
        ("anim_version", anim_version),
        ("anim_crc", anim_crc),
        ("anim_index", meta.anim_index.clone()),
        ("midi_version", midi_version),
        ("midi_crc", midi_crc),
        ("midi_index", finish(midi_index)),
        ("map_version", map_version),
        ("map_crc", map_crc),
        ("map_index", finish(map_index)),
    ];
    let mut jag = JagFile::new();
    for (name, data) in order_like(tables, &meta.order) {
        jag.write(name, data);
    }
    jag.build(JagArchive::VersionList.compression())
}

fn order_like<'a>(tables: Vec<(&'a str, Vec<u8>)>, order: &[String]) -> Vec<(&'a str, Vec<u8>)> {
    if order.is_empty() {
        return tables;
    }
    let mut tables = tables;
    tables.sort_by_key(|(name, _)| {
        order
            .iter()
            .position(|n| n.as_str() == *name)
            .unwrap_or(usize::MAX)
    });
    tables
}

fn finish(packet: Packet) -> Vec<u8> {
    packet.data[..packet.pos].to_vec()
}
