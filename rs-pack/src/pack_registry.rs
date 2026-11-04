//! Pack file registry — loads `.pack` files for name→id and id→name resolution.
//! Equivalent to 2004scape's `PackFile` class used by all config packers.

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use tracing::info;

pub struct PackFile {
    pub id_to_name: HashMap<u32, String>,
    pub name_to_id: HashMap<String, u32>,
    pub max: u32,
}

impl PackFile {
    pub fn empty() -> Self {
        Self {
            id_to_name: HashMap::new(),
            name_to_id: HashMap::new(),
            max: 0,
        }
    }

    pub fn load(path: &Path) -> Result<Self> {
        let mut id_to_name = HashMap::new();
        let mut name_to_id = HashMap::new();
        let mut max = 0u32;

        if let Ok(text) = std::fs::read_to_string(path) {
            for line in text.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                let Some((id_str, name)) = line.split_once('=') else {
                    continue;
                };
                let Ok(id) = id_str.parse::<u32>() else {
                    continue;
                };
                let name = name.trim().to_string();
                id_to_name.insert(id, name.clone());
                name_to_id.insert(name, id);
                if id >= max {
                    max = id + 1;
                }
            }
        }

        Ok(Self {
            id_to_name,
            name_to_id,
            max,
        })
    }

    pub fn get_by_name(&self, name: &str) -> Option<u32> {
        self.name_to_id.get(name).copied()
    }

    pub fn get_by_id(&self, id: u32) -> Option<&str> {
        self.id_to_name.get(&id).map(|s| s.as_str())
    }
}

/// Build the `"<iface_name>:<comp_name>" → component hash` map from interface.pack.
///
/// interface.pack interleaves interface lines (`<id>=<name>`) with component
/// lines (`<iface_id>:<comp_index>=<comp_name>`). This walks the component lines,
/// resolves the owning interface's *name* via `interfaces` (id→name), and keys
/// by the symbolic form scripts use (`chatbox:com_dialogue_box_small`).
fn load_component_hashes(path: &Path, interfaces: &PackFile) -> HashMap<String, i32> {
    let mut map = HashMap::new();
    let Ok(text) = std::fs::read_to_string(path) else {
        return map;
    };
    for line in text.lines() {
        let line = line.trim();
        let Some((id_str, comp_name)) = line.split_once('=') else {
            continue;
        };
        let Some((iface_str, comp_str)) = id_str.split_once(':') else {
            continue; // interface line, not a component
        };
        let (Ok(iface_id), Ok(comp_idx)) = (iface_str.parse::<u32>(), comp_str.parse::<u32>())
        else {
            continue;
        };
        let Some(iface_name) = interfaces.get_by_id(iface_id) else {
            continue;
        };
        let hash = ((iface_id << 16) | comp_idx) as i32;
        map.insert(format!("{iface_name}:{}", comp_name.trim()), hash);
    }
    map
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
    pub synth: PackFile,
    pub spotanim: PackFile,
    pub varp: PackFile,
    pub varbit: PackFile,
    pub varn: PackFile,
    pub inv: PackFile,
    pub idk: PackFile,
    pub r#enum: PackFile,
    pub r#struct: PackFile,
    pub mesanim: PackFile,
    pub vars: PackFile,
    pub model: PackFile,
    pub flo: PackFile,
    pub flu: PackFile,
    pub stat: PackFile,
    /// Songs and jingles under 289's one `midi` name space (a jingle at
    /// 10000 + its id at 647).
    pub midi: PackFile,
    /// Top-level interface ids (`<id>=<name>` lines of interface.pack). The
    /// `<iface>:<comp>=<name>` component lines are skipped (non-numeric id), so
    /// this resolves interface *names* (e.g. `objbox` → 519) for enum/param
    /// symbols typed `interface`/`component`.
    pub interface: PackFile,
    /// Component symbols `"<iface_name>:<comp_name>"` → component hash
    /// (`iface_id << 16 | comp_index`), built from interface.pack. Lets
    /// enum/param `component` values be written symbolically
    /// (e.g. `chatbox:com_dialogue_box_small`) instead of raw hashes.
    pub component_hashes: HashMap<String, i32>,
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
        let synth = load("synth")?;
        let spotanim = load("spotanim")?;
        let varp = load("varp")?;
        let varbit = load("varbit")?;
        let varn = load("varn")?;
        let inv = load("inv")?;
        let idk = load("idk")?;
        let r#enum = load("enum")?;
        let r#struct = load("struct")?;
        let mesanim = load("mesanim")?;
        let vars = load("vars")?;
        let model = load("model")?;
        let flo = load("flo")?;
        let flu = load("flu")?;
        let stat = load("stat")?;
        let midi = load("midi")?;
        let interface = load("interface")?;
        let component_hashes = load_component_hashes(&pack_dir.join("interface.pack"), &interface);

        info!(
            "PackRegistry: npc={} obj={} loc={} hunt={} param={} cat={} seq={} varp={}",
            npc.max, obj.max, loc.max, hunt.max, param.max, category.max, seq.max, varp.max,
        );

        Ok(Self {
            npc,
            obj,
            loc,
            hunt,
            param,
            category,
            seq,
            synth,
            spotanim,
            varp,
            varbit,
            varn,
            inv,
            idk,
            r#enum,
            r#struct,
            mesanim,
            vars,
            model,
            flo,
            flu,
            stat,
            midi,
            interface,
            component_hashes,
            pack_dir: pack_dir.to_path_buf(),
        })
    }

    pub fn pack_dir(&self) -> &Path {
        &self.pack_dir
    }
}
