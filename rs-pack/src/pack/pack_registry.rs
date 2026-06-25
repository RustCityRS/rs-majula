use std::collections::HashMap;
use std::path::Path;

use crate::pack::packed_data::PackedData;
use anyhow::Result;
use tracing::debug;

pub struct PackFile {
    pub id_to_debugname: HashMap<u16, String>,
    pub debugname_to_id: HashMap<String, u16>,
    pub max: u16,
}

impl PackFile {
    pub fn empty() -> Self {
        Self {
            id_to_debugname: HashMap::new(),
            debugname_to_id: HashMap::new(),
            max: 0,
        }
    }

    pub fn load(path: &Path) -> Result<Self> {
        let mut id_to_debugname = HashMap::new();
        let mut debugname_to_id = HashMap::new();
        let mut max = 0u16;

        if let Ok(text) = std::fs::read_to_string(path) {
            for line in text.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                let Some((id_str, debugname)) = line.split_once('=') else {
                    continue;
                };
                let Ok(id) = id_str.parse::<u16>() else {
                    continue;
                };
                let name = debugname.trim().to_string();
                id_to_debugname.insert(id, name.clone());
                debugname_to_id.insert(name, id);
                if id >= max {
                    max = id + 1;
                }
            }
        }

        Ok(Self {
            id_to_debugname,
            debugname_to_id,
            max,
        })
    }

    pub fn get_by_debugname(&self, debugname: &str) -> Option<u16> {
        self.debugname_to_id.get(debugname).copied()
    }

    pub fn get_by_id(&self, id: u16) -> Option<&str> {
        self.id_to_debugname.get(&id).map(|s| s.as_str())
    }
}

pub struct PackedFile {
    pub server: PackedData,
    pub client: Option<PackedData>,
}

/// All pack files loaded for cross-type resolution.
pub struct PackRegistry {
    pub npc: PackFile,
    pub obj: PackFile,
    pub loc: PackFile,
    pub hunt: PackFile,
    pub param: PackFile,
    pub category: PackFile,
    pub seq: PackFile,
    pub spotanim: PackFile,
    pub varp: PackFile,
    pub varn: PackFile,
    pub inv: PackFile,
    pub idk: PackFile,
    pub r#enum: PackFile,
    pub r#struct: PackFile,
    pub mesanim: PackFile,
    pub vars: PackFile,
    pub model: PackFile,
    pub interface: PackFile,
    pub dbrow: PackFile,
    pub synth: PackFile,
    pub flo: PackFile,
    pub texture: PackFile,
    pub anim: PackFile,
    pack_dir: std::path::PathBuf,
}

impl PackRegistry {
    pub fn load(pack_dir: &Path) -> Result<Self> {
        let load = |name: &str| -> Result<PackFile> {
            let path = pack_dir.join(format!("{name}.pack"));
            PackFile::load(&path)
        };

        let npc = load("npc")?;
        let obj = load("obj")?;
        let loc = load("loc")?;
        let hunt = load("hunt")?;
        let param = load("param")?;
        let category = load("category")?;
        let seq = load("seq")?;
        let spotanim = load("spotanim")?;
        let varp = load("varp")?;
        let varn = load("varn")?;
        let inv = load("inv")?;
        let idk = load("idk")?;
        let r#enum = load("enum")?;
        let r#struct = load("struct")?;
        let mesanim = load("mesanim")?;
        let vars = load("vars")?;
        let model = load("model")?;
        let interface = load("interface")?;
        let dbrow = load("dbrow")?;
        let synth = load("synth")?;
        let flo = load("flo")?;
        let texture = load("texture")?;
        let anim = load("anim")?;

        debug!(
            "PackRegistry: npc={} obj={} loc={} hunt={} param={} cat={} seq={} varp={} varn={} model={} interface={} dbrow={} synth={} flo={} texture={} spotanim={} anim={} idk={}",
            npc.max,
            obj.max,
            loc.max,
            hunt.max,
            param.max,
            category.max,
            seq.max,
            varp.max,
            varn.max,
            model.max,
            interface.max,
            dbrow.max,
            synth.max,
            flo.max,
            texture.max,
            spotanim.max,
            anim.max,
            idk.max,
        );

        Ok(Self {
            npc,
            obj,
            loc,
            hunt,
            param,
            category,
            seq,
            spotanim,
            varp,
            varn,
            inv,
            idk,
            r#enum,
            r#struct,
            mesanim,
            vars,
            model,
            interface,
            dbrow,
            synth,
            flo,
            texture,
            anim,
            pack_dir: pack_dir.to_path_buf(),
        })
    }

    pub fn pack_dir(&self) -> &Path {
        &self.pack_dir
    }
}
