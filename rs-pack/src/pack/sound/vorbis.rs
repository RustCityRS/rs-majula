use super::vorbis_setup;
use crate::archive::VORBIS;
use crate::pack::{
    names::AssetNames,
    text::{Number, TextCodec},
};
use anyhow::{Context, Result, ensure};
use rs_io::js5::Js5Store;
use std::{fs, path::Path, sync::OnceLock};

pub struct Setup {
    block_sizes: u8,
    packet: Vec<u8>,
}

impl Setup {
    pub fn new(data: &[u8], names: &AssetNames) -> Result<Self> {
        let (_, framing) = vorbis_setup::modes(data, names)?;
        let mut data = data.to_vec();
        if framing / 8 >= data.len() {
            data.push(0);
        }
        data[framing / 8] |= 1 << (framing % 8);
        let mut packet = b"\x05vorbis".to_vec();
        packet.extend_from_slice(&data[1..]);
        Ok(Self {
            block_sizes: data[0],
            packet,
        })
    }
}

fn crc(bytes: &[u8]) -> u32 {
    static TABLE: OnceLock<[u32; 256]> = OnceLock::new();
    let table = TABLE.get_or_init(|| {
        std::array::from_fn(|index| {
            let mut value = (index as u32) << 24;
            for _ in 0..8 {
                value = if value & 0x80000000 != 0 {
                    (value << 1) ^ 0x04c11db7
                } else {
                    value << 1
                };
            }
            value
        })
    });
    bytes.iter().fold(0, |value, &byte| {
        (value << 8) ^ table[((value >> 24) as u8 ^ byte) as usize]
    })
}

fn page(out: &mut Vec<u8>, packet: &[u8], sequence: u32, granule: u64, flags: u8) -> Result<()> {
    let count = packet.len() / 255 + 1;
    ensure!(count <= 255, "Vorbis packet exceeds one Ogg page");
    let start = out.len();
    out.extend_from_slice(b"OggS\0");
    out.push(flags);
    out.extend_from_slice(&granule.to_le_bytes());
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(&sequence.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.push(count as u8);
    out.extend(std::iter::repeat_n(255, count - 1));
    out.push((packet.len() % 255) as u8);
    out.extend_from_slice(packet);
    let checksum = crc(&out[start..]);
    out[start + 22..start + 26].copy_from_slice(&checksum.to_le_bytes());
    Ok(())
}

pub fn packets(data: &[u8]) -> Result<Vec<Vec<u8>>> {
    let mut pos = 0;
    let mut packets = Vec::new();
    let mut partial = Vec::new();
    let mut serial = None;
    let mut sequence = 0u32;
    while pos < data.len() {
        let header = data.get(pos..pos + 27).context("Truncated Ogg header")?;
        ensure!(&header[..5] == b"OggS\0", "Invalid Ogg header");
        let page_serial = u32::from_le_bytes(header[14..18].try_into()?);
        ensure!(
            serial.is_none_or(|s| s == page_serial),
            "Chained Ogg streams are not supported"
        );
        serial = Some(page_serial);
        ensure!(
            u32::from_le_bytes(header[18..22].try_into()?) == sequence,
            "Ogg page sequence mismatch"
        );
        sequence += 1;
        ensure!(
            (header[5] & 1 != 0) == !partial.is_empty(),
            "Invalid Ogg continuation"
        );
        let count = header[26] as usize;
        let sizes = data
            .get(pos + 27..pos + 27 + count)
            .context("Truncated Ogg segment table")?;
        let end = pos + 27 + count + sizes.iter().map(|v| usize::from(*v)).sum::<usize>();
        let mut page = data.get(pos..end).context("Truncated Ogg page")?.to_vec();
        let expected = u32::from_le_bytes(page[22..26].try_into()?);
        page[22..26].fill(0);
        ensure!(crc(&page) == expected, "Ogg page CRC mismatch");
        let mut offset = pos + 27 + count;
        for &size in sizes {
            partial.extend_from_slice(&data[offset..offset + size as usize]);
            offset += size as usize;
            if size < 255 {
                packets.push(std::mem::take(&mut partial));
            }
        }
        pos = end;
    }
    ensure!(partial.is_empty(), "Incomplete Ogg packet");
    ensure!(
        packets.len() >= 3
            && packets[0].starts_with(b"\x01vorbis")
            && packets[1].starts_with(b"\x03vorbis")
            && packets[2].starts_with(b"\x05vorbis"),
        "Expected Ogg Vorbis headers"
    );
    ensure!(
        packets[0].get(11) == Some(&1),
        "The 647 client requires mono Vorbis"
    );
    Ok(packets)
}

fn sample(data: &[u8]) -> Result<([i32; 4], Vec<Vec<u8>>)> {
    ensure!(data.len() >= 20, "Truncated Vorbis sample header");
    let header =
        std::array::from_fn(|i| i32::from_be_bytes(data[i * 4..i * 4 + 4].try_into().unwrap()));
    let count = u32::from_be_bytes(data[16..20].try_into()?) as usize;
    ensure!(count <= data.len() - 20, "Invalid Vorbis packet count");
    let mut packets = Vec::with_capacity(count);
    let mut pos = 20;
    for _ in 0..count {
        let mut length = 0usize;
        loop {
            let n = usize::from(*data.get(pos).context("Truncated Vorbis packet length")?);
            pos += 1;
            length += n;
            if n < 255 {
                break;
            }
        }
        packets.push(
            data.get(pos..pos + length)
                .context("Truncated Vorbis packet")?
                .to_vec(),
        );
        pos += length;
    }
    ensure!(pos == data.len(), "Unread Vorbis sample bytes");
    Ok((header, packets))
}

fn ogg(
    header: &[i32; 4],
    packets: &[Vec<u8>],
    setup: &[u8],
    modes: &[bool],
    framing: usize,
) -> Result<Vec<u8>> {
    ensure!(
        header[0] > 0 && header[1] >= 0,
        "Invalid Vorbis sample dimensions"
    );
    let mut out = Vec::new();
    let mut id = b"\x01vorbis".to_vec();
    id.extend_from_slice(&0u32.to_le_bytes());
    id.push(1);
    id.extend_from_slice(&header[0].to_le_bytes());
    id.extend_from_slice(&[0; 12]);
    id.extend_from_slice(&[setup[0], 1]);
    page(&mut out, &id, 0, 0, 2)?;
    let mut comment = b"\x03vorbis".to_vec();
    comment.extend_from_slice(&7u32.to_le_bytes());
    comment.extend_from_slice(b"rs-pack");
    comment.extend_from_slice(&0u32.to_le_bytes());
    comment.push(1);
    page(&mut out, &comment, 1, 0, 0)?;
    let mut setup = setup.to_vec();
    if framing / 8 >= setup.len() {
        setup.push(0);
    }
    setup[framing / 8] |= 1 << (framing % 8);
    let mut setup_packet = b"\x05vorbis".to_vec();
    setup_packet.extend_from_slice(&setup[1..]);
    page(&mut out, &setup_packet, 2, 0, 0)?;
    let mode_bits = 32 - (modes.len() as u32 - 1).leading_zeros();
    let mut granule = 0u64;
    let mut previous = 0;
    for (i, packet) in packets.iter().enumerate() {
        let first = *packet.first().context("Empty Vorbis audio packet")?;
        ensure!(first & 1 == 0, "Invalid Vorbis audio packet");
        let mode = usize::from((first >> 1) & ((1u16 << mode_bits) - 1) as u8);
        let large = *modes.get(mode).context("Invalid Vorbis mode")?;
        let size = 1u64 << if large { setup[0] >> 4 } else { setup[0] & 15 };
        if i > 0 {
            granule += (previous + size) / 4;
        }
        previous = size;
        let last = i + 1 == packets.len();
        page(
            &mut out,
            packet,
            i as u32 + 3,
            if last { header[1] as u64 } else { granule },
            if last { 4 } else { 0 },
        )?;
    }
    Ok(out)
}

pub fn pack_sample(text: &str, audio: &[u8], setup: &Setup, names: &AssetNames) -> Result<Vec<u8>> {
    let mut c = TextCodec::pack(text, names)?;
    let rate = c.number("sample_rate", Number::I32)? as i32;
    let count = c.number("sample_count", Number::I32)? as i32;
    let start = c.number("loop_start", Number::I32)? as i32;
    let end = c.number("loop_end", Number::I32)? as i32;
    let looped = c.label("ping_pong", "")?;
    ensure!(
        looped == "yes" || looped == "no",
        "Invalid Vorbis ping_pong flag"
    );
    ensure!(
        rate > 0 && count >= 0 && start >= 0 && end >= 0,
        "Invalid Vorbis sample dimensions"
    );
    c.finish()?;
    let packets = packets(audio)?;
    ensure!(
        packets[0].len() == 30 && packets[0][7..11] == [0; 4] && packets[0][29] == 1,
        "Invalid Vorbis identification header"
    );
    ensure!(
        packets[0][28] == setup.block_sizes && packets[2] == setup.packet,
        "Ogg codebooks or block sizes differ from the shared Vorbis setup; use audio encoded with the cache's shared setup"
    );
    let mut out = Vec::new();
    for n in [rate, count, start, if looped == "yes" { !end } else { end }] {
        out.extend_from_slice(&n.to_be_bytes());
    }
    out.extend_from_slice(&((packets.len() - 3) as u32).to_be_bytes());
    for packet in &packets[3..] {
        out.extend(std::iter::repeat_n(255, packet.len() / 255));
        out.push((packet.len() % 255) as u8);
        out.extend_from_slice(packet);
    }
    Ok(out)
}

pub fn unpack_archive(store: &mut Js5Store, pack_dir: &Path, content_dir: &Path) -> Result<u32> {
    let dir = content_dir.join("vorbis");
    fs::create_dir_all(&dir)?;
    let names = AssetNames::load(pack_dir)?;
    let setup = store
        .read_group_raw_decompressed(VORBIS, 0)?
        .context("Missing Vorbis setup")?;
    let text = vorbis_setup::unpack(&setup, &names)?;
    ensure!(
        vorbis_setup::pack(&text, &names)? == setup,
        "Vorbis setup round trip changed bytes"
    );
    fs::write(
        dir.join(format!("{}.vorbis", names.name("vorbis", 0))),
        text,
    )?;
    let (modes, framing) = vorbis_setup::modes(&setup, &names)?;
    let shared = Setup::new(&setup, &names)?;
    let groups = store
        .reference_table(VORBIS)
        .context("Missing Vorbis table")?
        .groups
        .clone();
    for info in &groups {
        if info.id == 0 {
            continue;
        }
        let data = store
            .read_group_raw_decompressed(VORBIS, info.id)?
            .context("Missing Vorbis sample")?;
        let (header, packets) = sample(&data)?;
        let text = format!(
            "sample_rate={}\nsample_count={}\nloop_start={}\nloop_end={}\nping_pong={}\n",
            header[0],
            header[1],
            header[2],
            if header[3] < 0 { !header[3] } else { header[3] },
            if header[3] < 0 { "yes" } else { "no" }
        );
        let audio = ogg(&header, &packets, &setup, &modes, framing)?;
        ensure!(
            pack_sample(&text, &audio, &shared, &names)? == data,
            "Vorbis {} round trip changed bytes",
            info.id
        );
        let name = names.name("vorbis", info.id);
        fs::write(dir.join(format!("{name}.sample")), text)?;
        fs::write(dir.join(format!("{name}.ogg")), audio)?;
    }
    Ok(groups.len() as u32)
}
