use std::path::Path;

use rs_io::Packet;
use rs_io::jag::JagFile;
use tracing::debug;

pub(crate) fn known_hashes() -> Vec<i32> {
    vec![JagFile::hash("sounds.dat")]
}

pub fn unpack_sounds(jag: &JagFile, output_dir: &Path, pack_dir: &Path) -> anyhow::Result<()> {
    let synth_dir = output_dir.join("synth");
    std::fs::create_dir_all(&synth_dir)?;

    let Some(data) = jag.read("sounds.dat") else {
        return Ok(());
    };

    let mut buf = Packet::from(data.data);
    let mut count = 0;
    let mut order_lines = Vec::new();

    while buf.remaining() >= 2 {
        let id = buf.g2();
        if id == 0xFFFF {
            break;
        }

        let data_start = buf.pos;
        parse_jagfx(&mut buf);

        let name = format!("synth_{id}");
        order_lines.push(id.to_string());

        std::fs::write(
            synth_dir.join(format!("{name}.synth")),
            &buf.data[data_start..buf.pos],
        )?;
        count += 1;
    }

    std::fs::write(pack_dir.join("synth.order"), order_lines.join("\n") + "\n")?;
    let max_id = order_lines
        .iter()
        .filter_map(|s| s.parse::<u16>().ok())
        .max()
        .unwrap_or(0);
    let mut pack_lines: Vec<String> = Vec::new();
    for id in 0..=max_id {
        pack_lines.push(format!("{id}=synth_{id}"));
    }
    std::fs::write(pack_dir.join("synth.pack"), pack_lines.join("\n") + "\n")?;

    debug!("Unpacked {} synths from sounds JAG", count);
    Ok(())
}

fn parse_jagfx(buf: &mut Packet) {
    for _ in 0..10 {
        if buf.remaining() <= 0 {
            return;
        }
        let check = buf.g1();
        if check == 0 {
            continue;
        }
        buf.pos -= 1;
        parse_tone(buf);
    }
    buf.g2(); // loopBegin
    buf.g2(); // loopEnd
}

fn parse_tone(buf: &mut Packet) {
    parse_envelope(buf); // frequencyBase
    parse_envelope(buf); // amplitudeBase

    if buf.remaining() > 0 && buf.data[buf.pos] != 0 {
        parse_envelope(buf); // frequencyModRate
        parse_envelope(buf); // frequencyModRange
    } else {
        buf.pos += 1;
    }

    if buf.remaining() > 0 && buf.data[buf.pos] != 0 {
        parse_envelope(buf); // amplitudeModRate
        parse_envelope(buf); // amplitudeModRange
    } else {
        buf.pos += 1;
    }

    if buf.remaining() > 0 && buf.data[buf.pos] != 0 {
        parse_envelope(buf); // release
        parse_envelope(buf); // attack
    } else {
        buf.pos += 1;
    }

    for _ in 0..10 {
        let volume = buf.gsmart1or2();
        if volume == 0 {
            break;
        }
        buf.gsmart1or2s(); // semitone
        buf.gsmart1or2(); // delay
    }

    buf.gsmart1or2(); // reverbDelay
    buf.gsmart1or2(); // reverbVolume
    buf.g2(); // length
    buf.g2(); // start

    // 289 appends a pole/zero filter to every tone.
    #[cfg(since_289)]
    parse_filter(buf);
}

#[cfg(since_289)]
fn parse_filter(buf: &mut Packet) {
    let pairs = buf.g1();
    if pairs == 0 {
        return;
    }
    let counts = [(pairs >> 4) as usize, (pairs & 0xF) as usize];
    let migration_start = buf.g2();
    let migration_end = buf.g2();
    let migrated = buf.g1() as u32;

    for &count in &counts {
        for _ in 0..count {
            buf.g2();
            buf.g2();
        }
    }
    for (channel, &count) in counts.iter().enumerate() {
        for i in 0..count {
            if migrated & (1 << (channel * 4) << i) != 0 {
                buf.g2();
                buf.g2();
            }
        }
    }
    if migrated != 0 || migration_end != migration_start {
        parse_envelope_segments(buf);
    }
}

#[cfg(since_289)]
fn parse_envelope_segments(buf: &mut Packet) {
    let num_segments = buf.g1() as usize;
    for _ in 0..num_segments {
        buf.g2();
        buf.g2();
    }
}

fn parse_envelope(buf: &mut Packet) {
    buf.g1(); // form
    buf.g4s(); // start
    buf.g4s(); // end
    let num_segments = buf.g1() as usize;
    for _ in 0..num_segments {
        buf.g2(); // shapeDelta
        buf.g2(); // shapePeak
    }
}
