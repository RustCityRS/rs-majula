use crate::pack::text::{Number, TextCodec};
use anyhow::Result;

pub fn transcode(c: &mut TextCodec<'_>) -> Result<()> {
    let key_runs = c.terminated("keygroup_runs")?;
    c.array("keygroups_plus_one", Number::I8, key_runs.len() + 1)?;
    let pan_runs = c.terminated("pan_runs")?;
    c.array("pan_offsets", Number::I8, pan_runs.len() + 1)?;
    let envelope_runs = c.terminated("envelope_runs")?;
    let mut count = if envelope_runs.is_empty() { 1 } else { 2 };
    for i in 2..=envelope_runs.len() {
        if c.number(&format!("envelope_mapping{i}"), Number::U8)? == 0 {
            count += 1;
        }
    }
    let mut volume = Vec::new();
    let mut release = Vec::new();
    for i in 0..count {
        volume.push(c.number(&format!("envelope{i}.volume_points"), Number::U8)? as usize);
        release.push(c.number(&format!("envelope{i}.release_points"), Number::U8)? as usize);
    }
    let volume_curve = c.number("volume_curve_points", Number::U8)? as usize;
    let pan_curve = c.number("pan_curve_points", Number::U8)? as usize;
    let sample_runs = c.terminated("sample_runs")?;
    c.array("pitch_low_deltas", Number::U8, 128)?;
    c.array("pitch_high_deltas", Number::U8, 128)?;
    let mut samples = Vec::new();
    let mut left = 0;
    let mut index = 0;
    let mut sample = 0;
    for note in 0..128 {
        if left == 0 {
            left = sample_runs.get(index).copied().unwrap_or(-1);
            index += 1;
            sample = c.sample(&format!("note{note}.sample"))?;
        }
        samples.push(sample);
        left -= 1;
    }
    left = 0;
    index = 0;
    for (note, &sample) in samples.iter().enumerate() {
        if left == 0 {
            left = sample_runs.get(index).copied().unwrap_or(-1);
            index += 1;
            if sample > 0 {
                c.number(&format!("note{note}.volume_minus_one"), Number::U8)?;
            }
        }
        left -= 1;
    }
    c.number("volume_minus_one", Number::U8)?;
    for i in 0..count {
        if volume[i] > 0 {
            c.array(&format!("envelope{i}.volume_values"), Number::I8, volume[i])?;
        }
        if release[i] > 0 {
            c.array(
                &format!("envelope{i}.release_values"),
                Number::I8,
                release[i] - 1,
            )?;
        }
    }
    if volume_curve > 0 {
        c.array("volume_curve_values", Number::I8, volume_curve)?;
    }
    if pan_curve > 0 {
        c.array("pan_curve_values", Number::I8, pan_curve)?;
    }
    for (i, &n) in release.iter().enumerate() {
        if n > 0 {
            c.array(&format!("envelope{i}.release_time_deltas"), Number::U8, n)?;
        }
    }
    for (i, &n) in volume.iter().enumerate() {
        if n > 0 {
            c.array(
                &format!("envelope{i}.volume_time_deltas"),
                Number::U8,
                n - 1,
            )?;
        }
    }
    if volume_curve > 0 {
        c.array("volume_curve_note_deltas", Number::U8, volume_curve)?;
    }
    if pan_curve > 0 {
        c.array("pan_curve_note_deltas", Number::U8, pan_curve)?;
    }
    let mut decay = Vec::new();
    for i in 0..count {
        decay.push(c.number(&format!("envelope{i}.decay"), Number::U8)?);
    }
    for i in 0..count {
        if volume[i] > 0 {
            c.number(&format!("envelope{i}.volume_keytrack"), Number::U8)?;
        }
        if release[i] > 0 {
            c.number(&format!("envelope{i}.release_keytrack"), Number::U8)?;
        }
        if decay[i] > 0 {
            c.number(&format!("envelope{i}.decay_keytrack"), Number::U8)?;
        }
    }
    let mut vibrato = Vec::new();
    for i in 0..count {
        vibrato.push(c.number(&format!("envelope{i}.vibrato_rate"), Number::U8)?);
    }
    let mut depth = Vec::new();
    for (i, &rate) in vibrato.iter().enumerate() {
        depth.push(if rate > 0 {
            c.number(&format!("envelope{i}.vibrato_depth"), Number::U8)?
        } else {
            0
        });
    }
    for (i, &depth) in depth.iter().enumerate() {
        if depth > 0 {
            c.number(&format!("envelope{i}.vibrato_delay"), Number::U8)?;
        }
    }
    Ok(())
}
