use anyhow::{Context, Result, ensure};
use rustc_hash::FxHashMap as HashMap;

type Fields = HashMap<String, String>;
type Arrays = HashMap<String, Vec<i64>>;

fn fields(text: &str) -> Result<Fields> {
    let mut result = Fields::default();
    for line in text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#') && !l.starts_with("//"))
    {
        let (key, value) = line.split_once('=').context("Invalid model property")?;
        ensure!(
            result
                .insert(key.trim().into(), value.trim().into())
                .is_none(),
            "Duplicate model property: {key}"
        );
    }
    Ok(result)
}

fn take(fields: &mut Fields, key: &str) -> Result<String> {
    fields
        .remove(key)
        .with_context(|| format!("Missing model property: {key}"))
}
fn number(fields: &Fields, key: &str) -> Result<usize> {
    Ok(fields
        .get(key)
        .with_context(|| format!("Missing model property: {key}"))?
        .parse()?)
}
fn array(fields: &mut Fields, key: &str) -> Result<Vec<i64>> {
    take(fields, key)?
        .split(',')
        .filter(|s| !s.is_empty())
        .map(|v| Ok(v.parse()?))
        .collect()
}
fn list(values: &[i64]) -> String {
    values
        .iter()
        .map(i64::to_string)
        .collect::<Vec<_>>()
        .join(",")
}
fn tuple(fields: &mut Fields, key: &str) -> Result<[i64; 3]> {
    array(fields, key)?
        .try_into()
        .map_err(|_| anyhow::anyhow!("Expected three coordinates: {key}"))
}
fn key_order(key: &str) -> (usize, usize, &str) {
    for (order, prefix) in [
        "vertex",
        "face",
        "mapping",
        "emitter",
        "effector",
        "billboard",
    ]
    .into_iter()
    .enumerate()
    {
        if let Some(tail) = key.strip_prefix(prefix) {
            let (id, property) = tail.split_once('.').unwrap_or((tail, ""));
            if let Ok(id) = id.parse() {
                return (order + 1, id, property);
            }
        }
    }
    (0, 0, key)
}
fn render(mut fields: Fields) -> Result<String> {
    let mut out = format!("format={}\n", take(&mut fields, "format")?);
    let mut entries: Vec<_> = fields.into_iter().collect();
    entries.sort_by(|a, b| key_order(&a.0).cmp(&key_order(&b.0)));
    let mut previous = (0, 0);
    for (key, value) in entries {
        let (group, index, _) = key_order(&key);
        if (group, index) != previous {
            out.push('\n');
            previous = (group, index);
        }
        out.push_str(&format!("{key}={value}\n"));
    }
    Ok(out)
}

fn strategy(previous: [i64; 3], face: [i64; 3]) -> i64 {
    if face[0] == previous[0] && face[1] == previous[2] {
        2
    } else if face[0] == previous[2] && face[1] == previous[1] {
        3
    } else if face[0] == previous[1] && face[1] == previous[0] {
        4
    } else {
        1
    }
}

fn compatible(previous: [i64; 3], face: [i64; 3], strategy: i64) -> bool {
    match strategy {
        1 => true,
        2 => face[0] == previous[0] && face[1] == previous[2],
        3 => face[0] == previous[2] && face[1] == previous[1],
        4 => face[0] == previous[1] && face[1] == previous[0],
        _ => false,
    }
}

const ATTRIBUTES: &[(&str, &str, &str)] = &[
    ("vertex_labels", "vertex", "skin"),
    ("face_types", "face", "type"),
    ("face_priority", "face", "priority"),
    ("face_labels", "face", "skin"),
    ("face_alpha", "face", "alpha"),
];

pub fn readable(text: &str) -> Result<String> {
    let mut f = fields(text)?;
    let vertices = number(&f, "vertices")?;
    let faces = number(&f, "faces")?;
    let masks = array(&mut f, "vertex_flags")?;
    let x = array(&mut f, "vertex_x_deltas")?;
    let y = array(&mut f, "vertex_y_deltas")?;
    let z = array(&mut f, "vertex_z_deltas")?;
    let mut axes = [x.into_iter(), y.into_iter(), z.into_iter()];
    let mut position = [0; 3];
    ensure!(masks.len() == vertices, "Invalid model vertex count");
    for (i, mask) in masks.into_iter().enumerate() {
        let mut required = 0;
        for axis in 0..3 {
            if mask & (1 << axis) != 0 {
                let delta = axes[axis]
                    .next()
                    .context("Missing model coordinate delta")?;
                position[axis] += delta;
                if delta != 0 {
                    required |= 1 << axis;
                }
            }
        }
        f.insert(format!("vertex{i}"), list(&position));
        if mask != required {
            f.insert(format!("vertex{i}.encoding"), mask.to_string());
        }
    }
    ensure!(
        axes.iter_mut().all(|a| a.next().is_none()),
        "Unused model coordinate deltas"
    );
    let strategies = array(&mut f, "face_strategy")?;
    ensure!(strategies.len() == faces, "Invalid model face count");
    let mut deltas = array(&mut f, "face_vertex_deltas")?.into_iter();
    let mut previous = [0; 3];
    let mut last = 0;
    for (i, encoding) in strategies.into_iter().enumerate() {
        let mut face = previous;
        match encoding {
            1 => {
                face[0] = last + deltas.next().context("Missing face vertex")?;
                face[1] = face[0] + deltas.next().context("Missing face vertex")?;
            }
            2 => face[1] = face[2],
            3 => face[0] = face[2],
            4 => face.swap(0, 1),
            _ => anyhow::bail!("Invalid model face encoding"),
        }
        face[2] = if encoding == 1 { face[1] } else { last }
            + deltas.next().context("Missing face vertex")?;
        f.insert(format!("face{i}.vertices"), list(&face));
        if strategy(previous, face) != encoding {
            f.insert(format!("face{i}.encoding"), encoding.to_string());
        }
        previous = face;
        last = face[2];
    }
    ensure!(deltas.next().is_none(), "Unused face vertex deltas");
    for &(key, kind, property) in ATTRIBUTES {
        if f.contains_key(key) {
            for (i, value) in array(&mut f, key)?.into_iter().enumerate() {
                f.insert(format!("{kind}{i}.{property}"), value.to_string());
            }
        }
    }
    render(f)
}

pub fn encoded(text: &str) -> Result<(Fields, Arrays)> {
    let mut f = fields(text)?;
    let mut arrays = Arrays::default();
    let vertices = number(&f, "vertices")?;
    let faces = number(&f, "faces")?;
    ensure!(
        vertices <= u16::MAX as usize && faces <= u16::MAX as usize,
        "Model vertex or face count exceeds 65535"
    );
    let mut masks = Vec::with_capacity(vertices);
    let mut axes = [Vec::new(), Vec::new(), Vec::new()];
    let mut previous = [0; 3];
    for i in 0..vertices {
        let vertex = tuple(&mut f, &format!("vertex{i}"))?;
        ensure!(
            vertex.iter().all(|&v| i32::try_from(v).is_ok()),
            "Vertex {i} coordinate exceeds signed 32-bit range"
        );
        let mut mask = f
            .remove(&format!("vertex{i}.encoding"))
            .map(|s| s.parse::<i64>())
            .transpose()?
            .unwrap_or(0);
        ensure!((0..=255).contains(&mask), "Invalid vertex encoding");
        for axis in 0..3 {
            if vertex[axis] != previous[axis] {
                mask |= 1 << axis;
            }
        }
        for axis in 0..3 {
            if mask & (1 << axis) != 0 {
                axes[axis].push(vertex[axis] - previous[axis]);
            }
        }
        masks.push(mask);
        previous = vertex;
    }
    arrays.insert("vertex_flags".into(), masks);
    for (name, values) in ["x", "y", "z"].into_iter().zip(axes) {
        arrays.insert(format!("vertex_{name}_deltas"), values);
    }
    let mut previous = [0; 3];
    let mut last = 0;
    let mut strategies = Vec::with_capacity(faces);
    let mut deltas = Vec::new();
    for i in 0..faces {
        let face = tuple(&mut f, &format!("face{i}.vertices"))?;
        ensure!(
            face.iter().all(|&v| v >= 0 && (v as usize) < vertices),
            "Face {i} references a missing vertex"
        );
        let requested = f
            .remove(&format!("face{i}.encoding"))
            .map(|s| s.parse::<i64>())
            .transpose()?;
        let encoding = requested
            .filter(|&s| compatible(previous, face, s))
            .unwrap_or_else(|| strategy(previous, face));
        if encoding == 1 {
            deltas.extend_from_slice(&[face[0] - last, face[1] - face[0], face[2] - face[1]]);
        } else {
            deltas.push(face[2] - last);
        }
        strategies.push(encoding);
        previous = face;
        last = face[2];
    }
    arrays.insert("face_strategy".into(), strategies);
    arrays.insert("face_vertex_deltas".into(), deltas);
    for &(key, kind, property) in ATTRIBUTES {
        let enabled = match key {
            "vertex_labels" => number(&f, "has_vertex_labels")? == 1,
            "face_labels" => number(&f, "has_face_labels")? == 1,
            "face_alpha" => number(&f, "has_alpha")? == 1,
            "face_priority" => number(&f, "priority")? == 255,
            "face_types" => {
                if f.get("format").is_some_and(|s| s == "new") {
                    number(&f, "flags")? & 1 != 0
                } else {
                    number(&f, "flags")? == 1
                }
            }
            _ => false,
        };
        if enabled {
            let count = if kind == "vertex" { vertices } else { faces };
            let mut values = Vec::with_capacity(count);
            for i in 0..count {
                values.push(take(&mut f, &format!("{kind}{i}.{property}"))?.parse()?);
            }
            arrays.insert(key.into(), values);
        }
    }
    Ok((f, arrays))
}
