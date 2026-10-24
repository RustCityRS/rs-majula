use crate::app::AppError;
use axum::response::Html;
use minijinja::{Environment, Error, ErrorKind, TemplateSource, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Every overlay, oldest first: a folder under `templates/` of the templates
/// that change from that revision on, laid over the ones before it.
pub const OVERLAYS: &[&str] = &["since_289"];

/// The overlays this build's revision uses, oldest first.
pub const ACTIVE_OVERLAYS: &[&str] = &[
    #[cfg(since_289)]
    "since_289",
];

/// The site's minijinja templates, loaded from disk.
///
/// - `layout/`: shared partials (header, footer, boxes)
/// - `pages/`: pages served straight from their URL (`/faq/billing` ->
///   `pages/faq/billing.html`)
/// - `views/`: pages rendered by a handler with data
/// - an [`OVERLAYS`] folder: the same tree, holding only the templates that
///   revision changes or adds; a name is looked up in the newest active
///   overlay first
///
/// Debug builds pick up template edits without a restart.
pub struct Templates {
    env: Environment<'static>,
}

impl Templates {
    /// `site_url` is the site's base URL, for the links a page gives of
    /// itself to other sites (its preview's image).
    pub fn new(dir: PathBuf, https: bool, site_url: &str) -> Self {
        Self::with_overlays(dir, ACTIVE_OVERLAYS, https, site_url)
    }

    /// [`Templates::new`] with `overlays` (oldest first) instead of this
    /// build's.
    fn with_overlays(dir: PathBuf, overlays: &[&str], https: bool, site_url: &str) -> Self {
        let mut env = Environment::new();
        let mut roots: Vec<PathBuf> = overlays.iter().rev().map(|o| dir.join(o)).collect();
        roots.push(dir);
        env.set_loader(move |name| load(&roots, name));
        env.set_auto_reload(cfg!(debug_assertions));
        env.add_global("https_enabled", https);
        env.add_global("site_url", site_url.to_owned());
        env.add_filter("commas", commas);
        Self { env }
    }

    pub fn render(&self, name: &str, ctx: Value) -> Result<Html<String>, AppError> {
        Ok(Html(self.env.get_template(name)?.render(ctx)?))
    }

    /// Renders `name` if it exists; `None` if there is no such template.
    pub fn render_if_exists(&self, name: &str) -> Result<Option<Html<String>>, AppError> {
        match self.env.get_template(name) {
            Ok(t) => Ok(Some(Html(t.render(())?))),
            Err(e) if e.kind() == ErrorKind::TemplateNotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}

/// `name` from the first of `roots` that has it. It's up to date while that
/// file is unchanged and no root before it has gained one of the same name,
/// so debug builds pick up edits and new overlay templates alike.
fn load(roots: &[PathBuf], name: &str) -> Result<Option<TemplateSource>, Error> {
    // Names come from includes and the page router, which only builds
    // plain segments; refuse anything else, as minijinja's own loader does,
    // and an overlay's folder named as if it were a template of the base.
    let bad = |seg: &str| seg.is_empty() || seg.starts_with('.') || seg.contains('\\');
    let overlay = name
        .split('/')
        .next()
        .is_some_and(|s| OVERLAYS.contains(&s));
    if overlay || name.split('/').any(bad) {
        return Ok(None);
    }
    let paths: Vec<PathBuf> = roots.iter().map(|root| root.join(name)).collect();
    for (i, path) in paths.iter().enumerate() {
        // Stamped before reading, so a change racing the read shows as stale.
        let Some(stamp) = file_stamp(path) else {
            continue;
        };
        let source = fs::read_to_string(path).map_err(|e| {
            Error::new(ErrorKind::InvalidOperation, "could not read template").with_source(e)
        })?;
        let path = path.clone();
        let shadows = paths[..i].to_vec();
        return Ok(Some(TemplateSource::new(source).with_uptodate_check(
            move || file_stamp(&path) == Some(stamp) && shadows.iter().all(|p| !p.is_file()),
        )));
    }
    Ok(None)
}

/// When a file was last changed and its size, if it's a file.
fn file_stamp(path: &Path) -> Option<(SystemTime, u64)> {
    let meta = fs::metadata(path).ok().filter(|m| m.is_file())?;
    Some((meta.modified().ok()?, meta.len()))
}

/// `1234567|commas` -> `1,234,567`.
fn commas(n: i64) -> String {
    let digits = n.unsigned_abs().to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3 + 1);
    if n < 0 {
        out.push('-');
    }
    for (i, d) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(d);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn commas_group_thousands() {
        assert_eq!(commas(0), "0");
        assert_eq!(commas(999), "999");
        assert_eq!(commas(1000), "1,000");
        assert_eq!(commas(13_034_431), "13,034,431");
        assert_eq!(commas(-1234), "-1,234");
    }

    /// Every template under `dir`, as loader names (`pages/faq/billing.html`),
    /// leaving out the overlay folders.
    fn template_names(root: &Path, dir: &Path, out: &mut Vec<String>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                if dir == root && OVERLAYS.iter().any(|o| path.ends_with(o)) {
                    continue;
                }
                template_names(root, &path, out);
            } else if path.extension().is_some_and(|e| e == "html") {
                let name = path.strip_prefix(root).unwrap().to_string_lossy();
                out.push(name.replace('\\', "/"));
            }
        }
    }

    /// Pages are served with no context, so each must render as-is: this
    /// catches syntax errors, missing includes and filters misapplied to
    /// undefined values. Views and partials need a handler's data, so they
    /// are only compiled. Checked for the base site and for each revision's
    /// stack of overlays, whichever revision this is built for.
    #[test]
    fn every_template_compiles_and_every_page_renders() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates");
        let mut failures = Vec::new();
        for active in 0..=OVERLAYS.len() {
            let overlays = &OVERLAYS[..active];
            let templates =
                Templates::with_overlays(root.clone(), overlays, false, "http://localhost:8000");
            let mut names = Vec::new();
            template_names(&root, &root, &mut names);
            for overlay in overlays {
                let dir = root.join(overlay);
                template_names(&dir, &dir, &mut names);
            }
            names.sort();
            names.dedup();
            assert!(names.iter().any(|n| n == "pages/index.html"));

            let site = overlays.last().copied().unwrap_or("base");
            failures.extend(names.iter().filter_map(|name| {
                let template = match templates.env.get_template(name) {
                    Ok(t) => t,
                    Err(e) => return Some(format!("{site}: {name}: {e:#}")),
                };
                if !name.starts_with("pages/") {
                    return None;
                }
                template
                    .render(())
                    .err()
                    .map(|e| format!("{site}: {name}: {e:#}"))
            }));
        }
        assert!(
            failures.is_empty(),
            "{}",
            failures.join(
                "
"
            )
        );
    }

    /// An overlay replaces the base template of the same name, and only for
    /// the revisions it's active in.
    #[test]
    fn overlays_win_over_the_base() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates");
        for overlay in OVERLAYS {
            let mut names = Vec::new();
            let dir = root.join(overlay);
            template_names(&dir, &dir, &mut names);
            let with = Templates::with_overlays(root.clone(), &[overlay], false, "");
            for name in names {
                let source = fs::read_to_string(dir.join(&name)).unwrap();
                assert_eq!(
                    with.env.get_template(&name).unwrap().source(),
                    source,
                    "{overlay}/{name}"
                );
            }
        }
        let base = Templates::with_overlays(root.clone(), &[], false, "");
        assert!(base.env.get_template("since_289/pages/index.html").is_err());
        assert!(base.env.get_template("../Cargo.toml").is_err());
    }
}
