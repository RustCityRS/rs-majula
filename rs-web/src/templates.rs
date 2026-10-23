use crate::app::AppError;
use axum::response::Html;
use minijinja::{Environment, ErrorKind, Value, path_loader};
use std::path::PathBuf;

/// The site's minijinja templates, loaded from disk.
///
/// - `layout/`: shared partials (header, footer, boxes)
/// - `pages/`: pages served straight from their URL (`/faq/billing` ->
///   `pages/faq/billing.html`)
/// - `views/`: pages rendered by a handler with data
///
/// Debug builds pick up template edits without a restart.
pub struct Templates {
    env: Environment<'static>,
}

impl Templates {
    /// `site_url` is the site's base URL, for the links a page gives of
    /// itself to other sites (its preview's image).
    pub fn new(dir: PathBuf, https: bool, site_url: &str) -> Self {
        let mut env = Environment::new();
        env.set_loader(path_loader(dir));
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

    /// Every template under `dir`, as loader names (`pages/faq/billing.html`).
    fn template_names(root: &Path, dir: &Path, out: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
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
    /// are only compiled.
    #[test]
    fn every_template_compiles_and_every_page_renders() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates");
        let templates = Templates::new(root.clone(), false, "http://localhost:8000");
        let mut names = Vec::new();
        template_names(&root, &root, &mut names);
        assert!(names.iter().any(|n| n == "pages/index.html"));

        let failures: Vec<String> = names
            .iter()
            .filter_map(|name| {
                let template = match templates.env.get_template(name) {
                    Ok(t) => t,
                    Err(e) => return Some(format!("{name}: {e:#}")),
                };
                if !name.starts_with("pages/") {
                    return None;
                }
                template.render(()).err().map(|e| format!("{name}: {e:#}"))
            })
            .collect();
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }
}
