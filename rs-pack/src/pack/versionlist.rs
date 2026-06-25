use rs_io::Packet;
use rs_io::crc::getcrc;
use rs_io::jag::JagFile;

use crate::version_list::VersionListMeta;
use rs_io::js5::Js5Store;

pub fn build_version_list(
    cache: &Js5Store,
    meta: &VersionListMeta,
    midi_jingles: &[bool],
) -> Vec<u8> {
    let build = |index: usize, versions: &[u16], crcs: &[i32]| -> (Vec<u8>, Vec<u8>) {
        let n = versions.len();
        let mut ver = Packet::new(n * 2 + 16);
        let mut crc = Packet::new(n * 4 + 16);
        for (id, &v) in versions.iter().enumerate() {
            ver.p2(v);
            match cache.read(index, id, false).filter(|d| !d.is_empty()) {
                Some(blob) => crc.p4(crc_no_version(&blob)),
                None => crc.p4(crcs.get(id).copied().unwrap_or(0)),
            }
        }
        (finish(ver), finish(crc))
    };

    let (model_version, model_crc) = build(1, &meta.model_version, &meta.model_crc);
    let (anim_version, anim_crc) = build(2, &meta.anim_version, &meta.anim_crc);
    let (midi_version, midi_crc) = build(3, &meta.midi_version, &meta.midi_crc);
    let (map_version, map_crc) = build(4, &meta.map_version, &meta.map_crc);

    let mut model_index = Packet::new(meta.model_version.len() + 16);
    for id in 0..meta.model_version.len() {
        model_index.p1(meta.model_flags.get(id).copied().unwrap_or(0));
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
    build_jag_whole(&order_like(tables, &meta.order))
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

fn build_jag_whole(files: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let raw_size: usize = files.iter().map(|(_, d)| d.len()).sum();
    let mut buf = Packet::new(2 + files.len() * 10 + raw_size);
    buf.p2(files.len() as u16);
    for (name, data) in files {
        buf.p4(JagFile::hash(name));
        buf.p3(data.len() as i32); // unpacked size
        buf.p3(data.len() as i32); // packed size == unpacked (compressed as a whole)
    }
    for (_, data) in files {
        buf.pdata(data, 0, data.len());
    }
    let compressed = rs_io::bz2::bz2_compress(&buf.data[..buf.pos], true);

    let mut jag = Packet::new(6 + compressed.len());
    jag.p3(buf.pos as i32); // unpacked total
    jag.p3(compressed.len() as i32); // packed total (< unpacked marks whole-archive)
    jag.pdata(&compressed, 0, compressed.len());
    jag.data[..jag.pos].to_vec()
}

fn crc_no_version(blob: &[u8]) -> i32 {
    let end = blob.len().saturating_sub(2);
    getcrc(blob, 0, end)
}

fn finish(packet: Packet) -> Vec<u8> {
    packet.data[..packet.pos].to_vec()
}
