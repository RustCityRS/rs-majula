use crate::archive;
use crate::pack::pack_registry::PackFile;
use crate::pack::util::walk;
use anyhow::{Context, Result, bail, ensure};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub struct AssetNames {
    packs: HashMap<String, PackFile>,
    components: HashMap<u32, HashMap<u32, String>>,
}

impl AssetNames {
    pub fn load(dir: &Path) -> Result<Self> {
        let mut packs = HashMap::new();
        for entry in fs::read_dir(dir)? {
            let path = entry?.path();
            if path.extension().is_some_and(|e| e == "pack") {
                let name = path
                    .file_stem()
                    .unwrap()
                    .to_str()
                    .context("Invalid pack name")?;
                packs.insert(name.to_owned(), PackFile::load(&path)?);
            }
        }
        let mut components: HashMap<u32, HashMap<u32, String>> = HashMap::new();
        if let Ok(text) = fs::read_to_string(dir.join("interface.pack")) {
            for line in text.lines() {
                let Some((id, name)) = line.split_once('=') else {
                    continue;
                };
                let Some((group, file)) = id.split_once(':') else {
                    continue;
                };
                components
                    .entry(group.parse()?)
                    .or_default()
                    .insert(file.parse()?, name.to_owned());
            }
        }
        Ok(Self { packs, components })
    }

    pub fn name(&self, kind: &str, id: u32) -> String {
        self.packs
            .get(kind)
            .and_then(|p| p.get_by_id(id))
            .map(str::to_owned)
            .unwrap_or_else(|| format!("{kind}_{id}"))
    }

    pub fn id(&self, kind: &str, name: &str) -> Result<u32> {
        if let Some(id) = self.packs.get(kind).and_then(|p| p.get_by_debugname(name)) {
            return Ok(id);
        }
        if let Ok(id) = name
            .strip_prefix(&format!("{kind}_"))
            .unwrap_or(name)
            .parse()
        {
            return Ok(id);
        }
        bail!("Unknown {kind}: {name}")
    }

    pub fn reference(&self, kind: &str, id: i64) -> String {
        if id < 0 {
            "null".to_owned()
        } else {
            self.name(kind, id as u32)
        }
    }

    pub fn resolve(&self, kind: &str, value: &str) -> Result<i64> {
        if value == "null" || value == "-1" {
            Ok(-1)
        } else {
            Ok(self.id(kind, value)? as i64)
        }
    }

    pub fn files(
        &self,
        dir: &Path,
        kind: &str,
        extensions: &[&str],
    ) -> Result<Vec<(u32, PathBuf)>> {
        let mut files = HashMap::new();
        for path in walk(dir) {
            if !path
                .extension()
                .and_then(|s| s.to_str())
                .is_some_and(|e| extensions.contains(&e))
            {
                continue;
            }
            let relative = path.strip_prefix(dir)?;
            let full = relative
                .to_str()
                .context("Invalid asset filename")?
                .replace('\\', "/");
            let name = relative.with_extension("");
            let name = name
                .to_str()
                .context("Invalid asset filename")?
                .replace('\\', "/");
            let id = self
                .id(kind, &full)
                .or_else(|_| self.id(kind, &name))
                .with_context(|| path.display().to_string())?;
            ensure!(
                files.insert(id, path).is_none(),
                "Duplicate {kind} asset {id}"
            );
        }
        let mut files: Vec<_> = files.into_iter().collect();
        files.sort_by_key(|(id, _)| *id);
        Ok(files)
    }

    pub fn component_name(&self, group: u32, file: u32) -> String {
        self.components
            .get(&group)
            .and_then(|v| v.get(&file))
            .cloned()
            .unwrap_or_else(|| format!("com{file}"))
    }

    pub fn component_id(&self, group: u32, name: &str) -> Result<u32> {
        if let Some((&id, _)) = self
            .components
            .get(&group)
            .and_then(|v| v.iter().find(|(_, value)| *value == name))
        {
            return Ok(id);
        }
        name.strip_prefix("com_")
            .or_else(|| name.strip_prefix("com"))
            .unwrap_or(name)
            .parse()
            .with_context(|| format!("Unknown component: {name}"))
    }
}

pub fn archive_name(id: u8) -> &'static str {
    match id {
        archive::ANIMS => "anim",
        archive::BASES => "base",
        archive::INTERFACES => "interface",
        archive::SYNTH => "synth",
        archive::MAPS => "map",
        archive::SONGS => "songs",
        archive::MODELS => "model",
        archive::SPRITES => "sprites",
        archive::TEXTURES => "texture",
        archive::BINARY => "binary",
        archive::JINGLES => "jingle",
        archive::CLIENTSCRIPT => "clientscript",
        archive::FONTMETRICS => "fontmetrics",
        archive::VORBIS => "vorbis",
        archive::MUSIC_SAMPLES => "instrument",
        archive::WORLDMAP_DATA => "worldmap_data",
        archive::QUICKCHAT => "quickchat",
        archive::QUICKCHAT_GLOBAL => "quickchat_global",
        archive::MATERIALS => "material",
        archive::PARTICLES => "particle",
        archive::DEFAULTS => "defaults",
        archive::BILLBOARDS => "billboard",
        archive::DLLS => "dlls",
        archive::SHADERS => "shaders",
        archive::LOADING_SPRITES | archive::LOADING_SPRITES_RAW => "loading_sprites",
        archive::LOADING_SCREENS => "loading_backgrounds",
        archive::WORLDMAP_LOCS => "worldmap_loc",
        _ => "asset",
    }
}
