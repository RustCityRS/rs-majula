use anyhow::{Context, Result, ensure};
use rs_io::js5::java_string_hash;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use tracing::info;

pub fn recover(
    pack_dir: &Path,
    kind: &str,
    dictionaries: &[PathBuf],
    max_index: u32,
) -> Result<usize> {
    ensure!(
        !kind.is_empty()
            && kind
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_'),
        "Invalid registry kind"
    );
    let path = pack_dir.join(format!("{kind}.pack"));
    let original = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error.into()),
    };
    let mut known = BTreeMap::new();
    for line in original.lines() {
        if let Some((id, name)) = line.split_once('=')
            && let Ok(id) = id.parse::<u32>()
        {
            known.insert(id, name.to_owned());
        }
    }
    let hashes = fs::read_to_string(pack_dir.join(format!("{kind}.hashes")))?;
    let mut targets: HashMap<i32, Vec<u32>> = HashMap::new();
    for line in hashes.lines() {
        let (id, hash) = line.split_once('=').context("Invalid name hash")?;
        let (id, hash) = (id.parse::<u32>()?, hash.parse::<i32>()?);
        if hash != -1
            && known
                .get(&id)
                .is_none_or(|name| name == &format!("{kind}_{id}"))
        {
            targets.entry(hash).or_default().push(id);
        }
    }
    let mut matches: HashMap<i32, BTreeSet<String>> = HashMap::new();
    let mut literal = BTreeSet::new();
    let mut expanded = HashMap::new();
    let mut consider = |name: &str, pattern: Option<(&str, u32)>| {
        let name = name.to_lowercase();
        let hash = java_string_hash(&name);
        if targets.contains_key(&hash) {
            if let Some((pattern, index)) = pattern {
                expanded.insert(name.clone(), (pattern.to_lowercase(), index));
            } else {
                literal.insert(name.clone());
            }
            matches.entry(hash).or_default().insert(name);
        }
    };
    for dictionary in dictionaries {
        for line in fs::read_to_string(dictionary)?.lines().map(str::trim) {
            if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
                continue;
            }
            let name = line.split_once('\t').map_or(line, |(_, name)| name);
            if name.contains('#') {
                for index in 0..=max_index {
                    consider(&name.replace('#', &index.to_string()), Some((name, index)));
                }
            } else {
                consider(name, None);
            }
        }
    }
    let supported: BTreeSet<_> = matches
        .values()
        .flatten()
        .filter(|name| {
            if literal.contains(*name) {
                return true;
            }
            let (pattern, index) = &expanded[*name];
            [index.checked_sub(1), index.checked_add(1)]
                .into_iter()
                .flatten()
                .any(|index| {
                    let neighbour = pattern.replace('#', &index.to_string());
                    known.values().any(|name| name == &neighbour)
                        || matches
                            .get(&java_string_hash(&neighbour))
                            .is_some_and(|names| names.contains(&neighbour))
                })
        })
        .cloned()
        .collect();
    let mut recovered = Vec::new();
    for (hash, names) in matches {
        let ids = &targets[&hash];
        if names.len() != 1 || ids.len() != 1 {
            info!("Unresolved {kind} hash {hash}: {ids:?} {names:?}");
            continue;
        }
        let name = names.into_iter().next().unwrap();
        if !supported.contains(&name) {
            info!("Uncorroborated {kind} candidate {}={name}", ids[0]);
            continue;
        }
        if known
            .iter()
            .any(|(&id, existing)| id != ids[0] && existing == &name)
        {
            continue;
        }
        recovered.push((ids[0], name));
    }
    recovered.sort();
    if recovered.is_empty() {
        return Ok(0);
    }
    let replacing: BTreeSet<_> = recovered.iter().map(|(id, _)| *id).collect();
    let mut text = original
        .lines()
        .filter(|line| {
            !line
                .split_once('=')
                .and_then(|(id, _)| id.parse::<u32>().ok())
                .is_some_and(|id| replacing.contains(&id))
        })
        .collect::<Vec<_>>()
        .join("\n");
    if !text.is_empty() {
        text.push('\n');
    }
    for (id, name) in &recovered {
        text.push_str(&format!("{id}={name}\n"));
        info!("Recovered {kind} {id}={name}");
    }
    fs::write(path, text)?;
    Ok(recovered.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process;
    use std::slice::from_ref;

    #[test]
    fn names_require_hash_matches_and_unambiguous_supported_candidates() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../target/name-recovery-test")
            .join(process::id().to_string());
        fs::create_dir_all(&dir).unwrap();
        let pack = dir.join("test.pack");
        let hashes = dir.join("test.hashes");
        let dictionary = dir.join("names.txt");
        fs::write(&pack, "1=kept\n8=icon,8\n9:2=component\n").unwrap();
        fs::write(
            &hashes,
            format!(
                "1={}\n2={}\n3={}\n4={}\n5={}\n",
                java_string_hash("replacement"),
                java_string_hash("named"),
                java_string_hash("icon,9"),
                java_string_hash("lonely,32"),
                java_string_hash("an")
            ),
        )
        .unwrap();
        assert_eq!(java_string_hash("an"), java_string_hash("c0"));
        fs::write(
            &dictionary,
            "replacement\n900\tnamed\nicon,#\nlonely,#\nan\nc0\n",
        )
        .unwrap();
        assert_eq!(recover(&dir, "test", from_ref(&dictionary), 40).unwrap(), 2);
        let text = fs::read_to_string(&pack).unwrap();
        assert!(text.contains("1=kept\n") && text.contains("9:2=component\n"));
        assert!(text.contains("2=named\n") && text.contains("3=icon,9\n"));
        assert!(!text.contains("4=") && !text.contains("5="));
        assert_eq!(recover(&dir, "test", from_ref(&dictionary), 40).unwrap(), 0);
        for path in [pack, hashes, dictionary] {
            fs::remove_file(path).unwrap();
        }
        fs::remove_dir(dir).unwrap();
    }
}
