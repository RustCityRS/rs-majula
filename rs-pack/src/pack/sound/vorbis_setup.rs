use crate::pack::{names::AssetNames, text::TextCodec};
use anyhow::{Context, Result, ensure};

struct Bits<'a> {
    input: Option<&'a [u8]>,
    output: Vec<u8>,
    pos: usize,
    text: TextCodec<'a>,
}

impl Bits<'_> {
    fn value(&mut self, key: &str, width: u32, bias: i64) -> Result<u32> {
        let mut value = 0u32;
        if let Some(input) = self.input {
            ensure!(
                self.pos + width as usize <= input.len() * 8,
                "Truncated Vorbis setup"
            );
            for bit in 0..width as usize {
                value |=
                    u32::from((input[(self.pos + bit) / 8] >> ((self.pos + bit) % 8)) & 1) << bit;
            }
        }
        let value = self
            .text
            .label(key, &(i64::from(value) + bias).to_string())?
            .parse::<i64>()?
            - bias;
        ensure!(
            value >= 0 && value < 1i64 << width,
            "Vorbis field out of range: {key}"
        );
        let value = value as u32;
        self.output
            .resize((self.pos + width as usize).div_ceil(8), 0);
        for bit in 0..width as usize {
            self.output[(self.pos + bit) / 8] |=
                (((value >> bit) & 1) as u8) << ((self.pos + bit) % 8);
        }
        self.pos += width as usize;
        Ok(value)
    }

    fn array(&mut self, key: &str, width: u32, count: usize, bias: i64) -> Result<Vec<u32>> {
        let mut values = Vec::new();
        if let Some(input) = self.input {
            ensure!(
                self.pos + width as usize * count <= input.len() * 8,
                "Truncated Vorbis array"
            );
            for index in 0..count {
                let mut value = 0u32;
                for bit in 0..width as usize {
                    let pos = self.pos + index * width as usize + bit;
                    value |= u32::from((input[pos / 8] >> (pos % 8)) & 1) << bit;
                }
                values.push((i64::from(value) + bias).to_string());
            }
        }
        let value = self.text.label(key, &values.join(","))?;
        let values = value
            .split(',')
            .filter(|v| !v.is_empty())
            .map(|v| Ok(v.parse::<i64>()? - bias))
            .collect::<Result<Vec<_>>>()?;
        ensure!(
            values.len() == count,
            "Vorbis array {key}: expected {count} values"
        );
        self.output
            .resize((self.pos + width as usize * count).div_ceil(8), 0);
        for &value in &values {
            ensure!(
                value >= 0 && value < 1i64 << width,
                "Vorbis array value out of range: {key}"
            );
            for bit in 0..width as usize {
                self.output[self.pos / 8] |= (((value >> bit) & 1) as u8) << (self.pos % 8);
                self.pos += 1;
            }
        }
        Ok(values.into_iter().map(|v| v as u32).collect())
    }

    fn reference(&mut self, key: &str, kind: &str, width: u32, bias: i64) -> Result<u32> {
        let mut value = 0u32;
        if let Some(input) = self.input {
            ensure!(
                self.pos + width as usize <= input.len() * 8,
                "Truncated Vorbis reference"
            );
            for bit in 0..width as usize {
                value |=
                    u32::from((input[(self.pos + bit) / 8] >> ((self.pos + bit) % 8)) & 1) << bit;
            }
        }
        let name = self.text.names.reference(kind, i64::from(value) + bias);
        let name = self.text.label(key, &name)?;
        let value = self.text.names.resolve(kind, &name)? - bias;
        ensure!(
            value >= 0 && value < 1i64 << width,
            "Vorbis reference out of range: {key}"
        );
        self.output
            .resize((self.pos + width as usize).div_ceil(8), 0);
        for bit in 0..width as usize {
            self.output[(self.pos + bit) / 8] |=
                (((value >> bit) & 1) as u8) << ((self.pos + bit) % 8);
        }
        self.pos += width as usize;
        Ok(value as u32)
    }
}

fn ilog(value: u32) -> u32 {
    32 - value.leading_zeros()
}

fn transcode(c: &mut Bits<'_>) -> Result<(Vec<bool>, usize)> {
    c.value("small_block_exponent", 4, 0)?;
    c.value("large_block_exponent", 4, 0)?;
    let books = c.value("codebooks", 8, 1)? + 1;
    for i in 0..books {
        let p = format!("codebook_{i}");
        ensure!(
            c.value(&format!("{p}.sync"), 24, 0)? == 0x564342,
            "Invalid Vorbis codebook signature"
        );
        let dimensions = c.value(&format!("{p}.dimensions"), 16, 0)?;
        let entries = c.value(&format!("{p}.entries"), 24, 0)?;
        ensure!(
            dimensions > 0 && entries <= 1_000_000,
            "Invalid Vorbis codebook dimensions"
        );
        let ordered = c.value(&format!("{p}.ordered"), 1, 0)?;
        if ordered != 0 {
            c.value(&format!("{p}.initial_length"), 5, 1)?;
            let mut offset = 0;
            let mut runs = 0;
            while offset < entries {
                offset += c.value(&format!("{p}.length_run"), ilog(entries - offset), 0)?;
                runs += 1;
                ensure!(
                    runs <= 32 && offset <= entries,
                    "Invalid Vorbis codeword lengths"
                );
            }
        } else if c.value(&format!("{p}.sparse"), 1, 0)? != 0 {
            for j in 0..entries {
                if c.value(&format!("{p}.entry{j}.present"), 1, 0)? != 0 {
                    c.value(&format!("{p}.entry{j}.length"), 5, 1)?;
                }
            }
        } else {
            c.array(&format!("{p}.lengths"), 5, entries as usize, 1)?;
        }
        let lookup = c.value(&format!("{p}.lookup_type"), 4, 0)?;
        ensure!(lookup <= 2, "Unsupported Vorbis lookup type");
        if lookup != 0 {
            for field in ["minimum", "delta"] {
                c.value(&format!("{p}.{field}.mantissa"), 21, 0)?;
                c.value(&format!("{p}.{field}.exponent"), 10, -788)?;
                c.value(&format!("{p}.{field}.negative"), 1, 0)?;
            }
            let width = c.value(&format!("{p}.value_bits"), 4, 1)?;
            let width = width + 1;
            c.value(&format!("{p}.sequence"), 1, 0)?;
            let count = if lookup == 2 {
                entries
                    .checked_mul(dimensions)
                    .context("Vorbis lookup overflow")?
            } else {
                let mut value = 1u32;
                while (value + 1)
                    .checked_pow(dimensions)
                    .is_some_and(|n| n <= entries)
                {
                    value += 1;
                }
                value
            };
            ensure!(count <= 1_000_000, "Vorbis lookup is too large");
            c.array(&format!("{p}.multiplicands"), width, count as usize, 0)?;
        }
    }
    let times = c.value("time_transforms", 6, 1)? + 1;
    c.array("time_types", 16, times as usize, 0)?;
    let floors = c.value("floors", 6, 1)? + 1;
    for i in 0..floors {
        let p = format!("floor_{i}");
        ensure!(
            c.value(&format!("{p}.type"), 16, 0)? == 1,
            "Unsupported Vorbis floor type"
        );
        let count = c.value(&format!("{p}.partitions"), 5, 0)?;
        let classes = c.array(&format!("{p}.partition_classes"), 4, count as usize, 0)?;
        let mut dimensions = Vec::new();
        for j in 0..classes.iter().max().map_or(0, |n| n + 1) {
            let q = format!("{p}.class{j}");
            dimensions.push(c.value(&format!("{q}.dimensions"), 3, 1)? + 1);
            let subclasses = c.value(&format!("{q}.subclasses"), 2, 0)?;
            if subclasses != 0 {
                c.reference(&format!("{q}.masterbook"), "codebook", 8, 0)?;
            }
            for _ in 0..1 << subclasses {
                c.reference(&format!("{q}.subclass_book"), "codebook", 8, -1)?;
            }
        }
        c.value(&format!("{p}.multiplier"), 2, 1)?;
        let width = c.value(&format!("{p}.range_bits"), 4, 0)?;
        let points = classes
            .iter()
            .map(|i| dimensions[*i as usize] as usize)
            .sum();
        c.array(&format!("{p}.x_coordinates"), width, points, 0)?;
    }
    let residues = c.value("residues", 6, 1)? + 1;
    for i in 0..residues {
        let p = format!("residue_{i}");
        c.value(&format!("{p}.type"), 16, 0)?;
        c.value(&format!("{p}.begin"), 24, 0)?;
        c.value(&format!("{p}.end"), 24, 0)?;
        c.value(&format!("{p}.partition_size"), 24, 1)?;
        let count = c.value(&format!("{p}.classifications"), 6, 1)? + 1;
        c.reference(&format!("{p}.classbook"), "codebook", 8, 0)?;
        let mut cascades = Vec::new();
        for j in 0..count {
            let low = c.value(&format!("{p}.class{j}.cascade_low"), 3, 0)?;
            let high = if c.value(&format!("{p}.class{j}.has_high"), 1, 0)? != 0 {
                c.value(&format!("{p}.class{j}.cascade_high"), 5, 0)?
            } else {
                0
            };
            cascades.push(low | high << 3);
        }
        for (j, cascade) in cascades.into_iter().enumerate() {
            for pass in 0..8 {
                if cascade & 1 << pass != 0 {
                    c.reference(&format!("{p}.class{j}.pass{pass}"), "codebook", 8, 0)?;
                }
            }
        }
    }
    let mappings = c.value("mappings", 6, 1)? + 1;
    for i in 0..mappings {
        let p = format!("mapping_{i}");
        c.value(&format!("{p}.type"), 16, 0)?;
        let submaps = if c.value(&format!("{p}.multiple_submaps"), 1, 0)? != 0 {
            c.value(&format!("{p}.submaps"), 4, 1)? + 1
        } else {
            1
        };
        if c.value(&format!("{p}.coupling"), 1, 0)? != 0 {
            c.value(&format!("{p}.coupling_steps"), 8, 1)?;
        }
        c.value(&format!("{p}.reserved"), 2, 0)?;
        if submaps > 1 {
            c.value(&format!("{p}.mux"), 4, 0)?;
        }
        for j in 0..submaps {
            c.value(&format!("{p}.submap{j}.time"), 8, 0)?;
            c.reference(&format!("{p}.submap{j}.floor"), "floor", 8, 0)?;
            c.reference(&format!("{p}.submap{j}.residue"), "residue", 8, 0)?;
        }
    }
    let count = c.value("modes", 6, 1)? + 1;
    let mut modes = Vec::new();
    for i in 0..count {
        let p = format!("mode_{i}");
        modes.push(c.value(&format!("{p}.large_block"), 1, 0)? != 0);
        c.value(&format!("{p}.window"), 16, 0)?;
        c.value(&format!("{p}.transform"), 16, 0)?;
        c.reference(&format!("{p}.mapping"), "mapping", 8, 0)?;
    }
    let framing = c.pos;
    let padding = (8 - c.pos % 8) % 8;
    if padding != 0 {
        c.value("padding", padding as u32, 0)?;
    }
    Ok((modes, framing))
}

pub fn unpack(data: &[u8], names: &AssetNames) -> Result<String> {
    let mut codec = Bits {
        input: Some(data),
        output: Vec::new(),
        pos: 0,
        text: TextCodec::unpack(&[], names),
    };
    transcode(&mut codec)?;
    ensure!(
        codec.output == data,
        "Vorbis setup round trip changed bytes"
    );
    Ok(codec.text.finish()?.0)
}

pub fn pack(text: &str, names: &AssetNames) -> Result<Vec<u8>> {
    let mut codec = Bits {
        input: None,
        output: Vec::new(),
        pos: 0,
        text: TextCodec::pack(text, names)?,
    };
    transcode(&mut codec)?;
    codec.text.finish()?;
    Ok(codec.output)
}

pub fn modes(data: &[u8], names: &AssetNames) -> Result<(Vec<bool>, usize)> {
    let mut codec = Bits {
        input: Some(data),
        output: Vec::new(),
        pos: 0,
        text: TextCodec::unpack(&[], names),
    };
    transcode(&mut codec)
}
