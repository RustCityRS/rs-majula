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
    pub fn new(dir: PathBuf, https: bool) -> Self {
        let mut env = Environment::new();
        env.set_loader(path_loader(dir));
        env.set_auto_reload(cfg!(debug_assertions));
        env.add_global("https_enabled", https);
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
