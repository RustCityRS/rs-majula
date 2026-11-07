use crate::pack::names::AssetNames;
use anyhow::{Context, Result, bail, ensure};
use rs_io::Packet;
use rustc_hash::FxHashMap as HashMap;
use std::collections::VecDeque;
use std::fmt::Write;

#[derive(Clone, Copy)]
pub enum Number {
    U8,
    I8,
    U16,
    U24,
    I24,
    I16,
    U32,
    I32,
    Smart,
    SignedSmart,
    VarInt,
}

pub struct TextCodec<'a> {
    source: Option<&'a [u8]>,
    pos: usize,
    fields: HashMap<String, VecDeque<String>>,
    arrays: HashMap<String, Vec<i64>>,
    pub text: String,
    pub data: Vec<u8>,
    pub names: &'a AssetNames,
}

impl<'a> TextCodec<'a> {
    pub(crate) fn from_fields(
        fields: HashMap<String, String>,
        arrays: HashMap<String, Vec<i64>>,
        names: &'a AssetNames,
    ) -> Self {
        Self {
            source: None,
            pos: 0,
            fields: fields
                .into_iter()
                .map(|(key, value)| (key, VecDeque::from([value])))
                .collect(),
            arrays,
            text: String::new(),
            data: Vec::new(),
            names,
        }
    }

    pub fn unpack(data: &'a [u8], names: &'a AssetNames) -> Self {
        Self {
            source: Some(data),
            pos: 0,
            fields: HashMap::default(),
            arrays: HashMap::default(),
            text: String::new(),
            data: Vec::new(),
            names,
        }
    }

    pub fn pack(text: &str, names: &'a AssetNames) -> Result<Self> {
        let mut fields: HashMap<String, VecDeque<String>> = HashMap::default();
        for (i, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
                continue;
            }
            let (key, value) = line
                .split_once('=')
                .with_context(|| format!("Line {}: expected key=value", i + 1))?;
            fields
                .entry(key.trim().to_owned())
                .or_default()
                .push_back(value.trim().to_owned());
        }
        Ok(Self {
            source: None,
            pos: 0,
            fields,
            arrays: HashMap::default(),
            text: String::new(),
            data: Vec::new(),
            names,
        })
    }

    pub fn finish(self) -> Result<(String, Vec<u8>)> {
        if let Some(data) = self.source {
            ensure!(
                self.pos == data.len(),
                "{} unread asset bytes",
                data.len() - self.pos
            );
        } else {
            ensure!(self.arrays.is_empty(), "Unrecognized derived asset arrays");
            ensure!(
                self.fields.values().all(VecDeque::is_empty),
                "Unrecognized or duplicate asset properties: {:?}",
                self.fields
                    .iter()
                    .filter(|(_, v)| !v.is_empty())
                    .map(|(k, _)| k)
                    .collect::<Vec<_>>()
            );
        }
        Ok((self.text, self.data))
    }

    fn value(&mut self, key: &str) -> Result<String> {
        self.fields
            .get_mut(key)
            .and_then(VecDeque::pop_front)
            .with_context(|| format!("Missing asset property: {key}"))
    }

    fn read(&mut self, count: usize) -> Result<&'a [u8]> {
        let source = self.source.context("Not decoding an asset")?;
        let end = self.pos.checked_add(count).context("Asset size overflow")?;
        let result = source.get(self.pos..end).context("Truncated asset")?;
        self.pos = end;
        Ok(result)
    }

    fn read_number(&mut self, kind: Number) -> Result<i64> {
        Ok(match kind {
            Number::U8 => self.read(1)?[0] as i64,
            Number::I8 => self.read(1)?[0] as i8 as i64,
            Number::U16 => u16::from_be_bytes(self.read(2)?.try_into()?) as i64,
            Number::U24 => {
                let b = self.read(3)?;
                ((b[0] as i64) << 16) | ((b[1] as i64) << 8) | b[2] as i64
            }
            Number::I24 => {
                let b = self.read(3)?;
                (((b[0] as i32) << 24 | (b[1] as i32) << 16 | (b[2] as i32) << 8) >> 8) as i64
            }
            Number::I16 => i16::from_be_bytes(self.read(2)?.try_into()?) as i64,
            Number::U32 => u32::from_be_bytes(self.read(4)?.try_into()?) as i64,
            Number::I32 => i32::from_be_bytes(self.read(4)?.try_into()?) as i64,
            Number::Smart | Number::SignedSmart => {
                let first = self.read(1)?[0];
                let signed = matches!(kind, Number::SignedSmart);
                if first < 128 {
                    first as i64 - if signed { 64 } else { 0 }
                } else {
                    ((first as i64) << 8 | self.read(1)?[0] as i64)
                        - if signed { 49152 } else { 32768 }
                }
            }
            Number::VarInt => {
                let mut value = 0i64;
                loop {
                    let b = self.read(1)?[0];
                    ensure!(value <= 0x1ffffff, "Varint overflow");
                    value = (value << 7) | (b & 127) as i64;
                    if b & 128 == 0 {
                        break;
                    }
                }
                value
            }
        })
    }

    fn write_number(&mut self, kind: Number, value: i64) -> Result<()> {
        match kind {
            Number::U8 => self.data.push(u8::try_from(value)?),
            Number::I8 => self.data.push(i8::try_from(value)? as u8),
            Number::U16 => self
                .data
                .extend_from_slice(&u16::try_from(value)?.to_be_bytes()),
            Number::U24 => {
                ensure!((0..=0xffffff).contains(&value), "24-bit value out of range");
                self.data
                    .extend_from_slice(&(value as u32).to_be_bytes()[1..]);
            }
            Number::I24 => {
                ensure!(
                    (-0x800000..=0x7fffff).contains(&value),
                    "Signed 24-bit value out of range"
                );
                self.data
                    .extend_from_slice(&(value as i32).to_be_bytes()[1..]);
            }
            Number::I16 => self
                .data
                .extend_from_slice(&i16::try_from(value)?.to_be_bytes()),
            Number::U32 => self
                .data
                .extend_from_slice(&u32::try_from(value)?.to_be_bytes()),
            Number::I32 => self
                .data
                .extend_from_slice(&i32::try_from(value)?.to_be_bytes()),
            Number::Smart => {
                ensure!((0..32768).contains(&value), "Smart out of range: {value}");
                if value < 128 {
                    self.data.push(value as u8);
                } else {
                    self.data
                        .extend_from_slice(&(value as u16 + 32768).to_be_bytes());
                }
            }
            Number::SignedSmart => {
                ensure!(
                    (-16384..16384).contains(&value),
                    "Signed smart out of range: {value}"
                );
                if (-64..64).contains(&value) {
                    self.data.push((value + 64) as u8);
                } else {
                    self.data
                        .extend_from_slice(&((value + 49152) as u16).to_be_bytes());
                }
            }
            Number::VarInt => {
                let value = u32::try_from(value)?;
                for shift in [28, 21, 14, 7, 0] {
                    if value >> shift != 0 || shift == 0 {
                        self.data.push(
                            ((value >> shift) & 127) as u8 | if shift == 0 { 0 } else { 128 },
                        );
                    }
                }
            }
        }
        Ok(())
    }

    pub fn number(&mut self, key: &str, kind: Number) -> Result<i64> {
        if self.source.is_some() {
            let value = self.read_number(kind)?;
            writeln!(self.text, "{key}={value}")?;
            Ok(value)
        } else {
            let text = self.value(key)?;
            let value = if let Some(hex) = text.strip_prefix("0x") {
                i64::from_str_radix(hex, 16)?
            } else {
                text.parse()?
            };
            self.write_number(kind, value)
                .with_context(|| format!("{key}={text}"))?;
            Ok(value)
        }
    }

    pub fn coordinate(&mut self, key: &str) -> Result<()> {
        if self.source.is_some() {
            let value = self.read_number(Number::U32)?;
            writeln!(
                self.text,
                "{key}={},{},{}",
                value >> 28,
                value >> 14 & 0x3fff,
                value & 0x3fff
            )?;
        } else {
            let text = self.value(key)?;
            let parts = text
                .split(',')
                .map(|s| s.trim().parse::<u32>())
                .collect::<std::result::Result<Vec<_>, _>>()?;
            ensure!(
                parts.len() == 3 && parts[0] < 16 && parts[1] < 16384 && parts[2] < 16384,
                "{key}: expected plane,x,z"
            );
            self.write_number(
                Number::U32,
                ((parts[0] << 28) | (parts[1] << 14) | parts[2]) as i64,
            )?;
        }
        Ok(())
    }

    pub fn mapped(
        &mut self,
        key: &str,
        kind: Number,
        decode: impl FnOnce(i64) -> Result<String>,
        encode: impl FnOnce(&str) -> Result<i64>,
    ) -> Result<i64> {
        if self.source.is_some() {
            let value = self.read_number(kind)?;
            writeln!(self.text, "{key}={}", decode(value)?)?;
            Ok(value)
        } else {
            let value = encode(&self.value(key)?)?;
            self.write_number(kind, value)?;
            Ok(value)
        }
    }

    pub fn reference(&mut self, key: &str, kind: Number, domain: &str) -> Result<i64> {
        let null = match kind {
            Number::U8 => 255,
            Number::U16 => 65535,
            Number::U32 => u32::MAX as i64,
            _ => -1,
        };
        if self.source.is_some() {
            let value = self.read_number(kind)?;
            writeln!(
                self.text,
                "{key}={}",
                self.names
                    .reference(domain, if value == null { -1 } else { value })
            )?;
            Ok(value)
        } else {
            let text = self.value(key)?;
            let value = self.names.resolve(domain, &text)?;
            let value = if value == -1 { null } else { value };
            self.write_number(kind, value)?;
            Ok(value)
        }
    }

    pub fn array(&mut self, key: &str, kind: Number, count: usize) -> Result<Vec<i64>> {
        if let Some(values) = self.arrays.remove(key) {
            ensure!(
                values.len() == count,
                "{key}: expected {count} values, got {}",
                values.len()
            );
            for &value in &values {
                self.write_number(kind, value)?;
            }
            return Ok(values);
        }
        if self.source.is_some() {
            let values = (0..count)
                .map(|_| self.read_number(kind))
                .collect::<Result<Vec<_>>>()?;
            writeln!(
                self.text,
                "{key}={}",
                values
                    .iter()
                    .map(i64::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            )?;
            Ok(values)
        } else {
            let text = self.value(key)?;
            let values = if text.is_empty() {
                Vec::new()
            } else {
                text.split(',')
                    .map(|s| s.trim().parse())
                    .collect::<std::result::Result<Vec<i64>, _>>()?
            };
            ensure!(
                values.len() == count,
                "{key}: expected {count} values, got {}",
                values.len()
            );
            for &value in &values {
                self.write_number(kind, value)?;
            }
            Ok(values)
        }
    }

    pub fn string(&mut self, key: &str) -> Result<String> {
        if self.source.is_some() {
            let mut bytes = Vec::new();
            loop {
                let b = self.read(1)?[0];
                if b == 0 {
                    break;
                }
                bytes.push(b);
            }
            bytes.push(0);
            let value = Packet::from(bytes).gjstr(0);
            writeln!(self.text, "{key}={}", serde_json::to_string(&value)?)?;
            Ok(value)
        } else {
            let value: String = serde_json::from_str(&self.value(key)?)?;
            ensure!(!value.contains('\0'), "NUL in asset string");
            let mut packet = Packet::new(value.len() + 1);
            packet.pjstr(&value, 0);
            self.data.extend_from_slice(&packet.data[..packet.pos]);
            Ok(value)
        }
    }

    pub fn opcode(&mut self, options: &[(u8, &str)]) -> Result<u8> {
        if self.source.is_some() {
            let code = self.read_number(Number::U8)? as u8;
            let name = options
                .iter()
                .find(|&&(id, _)| id == code)
                .map(|&(_, name)| name)
                .with_context(|| format!("Unknown asset opcode {code}"))?;
            writeln!(self.text, "opcode={name}")?;
            Ok(code)
        } else {
            let name = self.value("opcode")?;
            let Some(&(code, _)) = options.iter().find(|&&(_, n)| n == name) else {
                bail!("Unknown asset opcode {name}");
            };
            self.data.push(code);
            Ok(code)
        }
    }

    pub fn offset(&self) -> usize {
        if self.source.is_some() {
            self.pos
        } else {
            self.data.len()
        }
    }

    pub fn length(&mut self) -> Result<usize> {
        let offset = self.offset();
        if self.source.is_some() {
            self.read_number(Number::U16)?;
        } else {
            self.data.extend_from_slice(&[0, 0]);
        }
        Ok(offset)
    }

    pub fn set_length(&mut self, offset: usize, start: usize) -> Result<()> {
        let len = u16::try_from(self.offset() - start)?;
        if let Some(source) = self.source {
            ensure!(
                source[offset..offset + 2] == len.to_be_bytes(),
                "Asset section length mismatch at {offset}"
            );
        } else {
            self.data[offset..offset + 2].copy_from_slice(&len.to_be_bytes());
        }
        Ok(())
    }

    pub fn reference_offset(
        &mut self,
        key: &str,
        kind: Number,
        domain: &str,
        bias: i64,
    ) -> Result<i64> {
        if self.source.is_some() {
            let value = self.read_number(kind)? - bias;
            writeln!(self.text, "{key}={}", self.names.reference(domain, value))?;
            Ok(value)
        } else {
            let text = self.value(key)?;
            let value = self.names.resolve(domain, &text)?;
            self.write_number(kind, value + bias)?;
            Ok(value)
        }
    }

    pub fn more(&self, key: &str) -> bool {
        self.source.map_or_else(
            || self.fields.get(key).is_some_and(|v| !v.is_empty()),
            |s| self.pos < s.len(),
        )
    }

    pub fn terminated(&mut self, key: &str) -> Result<Vec<i64>> {
        if self.source.is_some() {
            let mut values = Vec::new();
            loop {
                let value = self.read_number(Number::I8)?;
                if value == 0 {
                    break;
                }
                values.push(value);
            }
            writeln!(
                self.text,
                "{key}={}",
                values
                    .iter()
                    .map(i64::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            )?;
            Ok(values)
        } else {
            let text = self.value(key)?;
            let values = if text.is_empty() {
                Vec::new()
            } else {
                text.split(',')
                    .map(|v| v.trim().parse())
                    .collect::<std::result::Result<Vec<i64>, _>>()?
            };
            for &value in &values {
                ensure!(value != 0, "Zero in {key}");
                self.write_number(Number::I8, value)?;
            }
            self.data.push(0);
            Ok(values)
        }
    }

    pub fn sample(&mut self, key: &str) -> Result<i64> {
        if self.source.is_some() {
            let value = self.read_number(Number::VarInt)?;
            let text = if value == 0 {
                "null".to_owned()
            } else {
                let domain = if (value - 1) & 1 != 0 {
                    "vorbis"
                } else {
                    "synth"
                };
                format!(
                    "{domain}:{},{}",
                    self.names.name(domain, ((value - 1) >> 2) as u32),
                    if (value - 1) & 2 != 0 { "loop" } else { "once" }
                )
            };
            writeln!(self.text, "{key}={text}")?;
            Ok(value)
        } else {
            let text = self.value(key)?;
            let value = if text == "null" {
                0
            } else {
                let (name, mode) = text.split_once(',').context("Expected sample,once|loop")?;
                ensure!(matches!(mode, "once" | "loop"), "Invalid sample loop mode");
                let (domain, name) = name
                    .split_once(':')
                    .context("Expected synth:name or vorbis:name")?;
                ensure!(
                    matches!(domain, "synth" | "vorbis"),
                    "Unknown sample type: {domain}"
                );
                let flag = i64::from(domain == "vorbis");
                (self.names.id(domain, name)? as i64) * 4
                    + flag
                    + if mode == "loop" { 2 } else { 0 }
                    + 1
            };
            self.write_number(Number::VarInt, value)?;
            Ok(value)
        }
    }

    pub fn choice(&mut self, key: &str, kind: Number, options: &[&str]) -> Result<usize> {
        if self.source.is_some() {
            let value = self.read_number(kind)? as usize;
            let name = options
                .get(value)
                .with_context(|| format!("Unknown {key} {value}"))?;
            writeln!(self.text, "{key}={name}")?;
            Ok(value)
        } else {
            let text = self.value(key)?;
            let value = options
                .iter()
                .position(|&v| v == text)
                .with_context(|| format!("Unknown {key}: {text}"))?;
            self.write_number(kind, value as i64)?;
            Ok(value)
        }
    }

    pub fn source_len(&self) -> Option<usize> {
        self.source.map(<[u8]>::len)
    }

    pub fn label(&mut self, key: &str, value: &str) -> Result<String> {
        if self.source.is_some() {
            writeln!(self.text, "{key}={value}")?;
            Ok(value.to_owned())
        } else {
            self.value(key)
        }
    }
}
