use std::sync::OnceLock;

use minijinja::{context, Environment};
use poem::{error::InternalServerError, web::Html};
use rust_embed::RustEmbed;
use serde::Serialize;

mod context;
mod functions;

pub use context::*;

#[derive(RustEmbed)]
#[folder = "$CARGO_MANIFEST_DIR/templates/"]
struct TemplatesEmbed;

fn get_templates() -> &'static Environment<'static> {
    static TEMPLATES: OnceLock<Environment<'static>> = OnceLock::new();
    TEMPLATES.get_or_init(|| {
        let mut environment = Environment::new();

        tracing::info!("Loading embedded templates");
        for path in TemplatesEmbed::iter() {
            if let Some(file) = TemplatesEmbed::get(&path) {
                if let Ok(content) = std::str::from_utf8(file.data.as_ref()) {
                    if let Err(err) =
                        environment.add_template_owned(path.clone(), content.to_owned())
                    {
                        tracing::error!("Failed to load embedded template {}: {}", path, err);
                    }
                }
            }
        }

        functions::add_to_environment(&mut environment);

        environment
    })
}

pub fn render_template<S: Serialize>(name: &str, context: S) -> poem::Result<Html<String>> {
    let result = get_templates()
        .get_template(name)
        .map_err(|err| {
            tracing::error!(template_name = ?name, "Template render failed: {err:?}");
            InternalServerError(err)
        })?
        .render(context)
        .map_err(|err| {
            tracing::error!(template_name = ?name, "Template render feailed: {err:?}");
            InternalServerError(err)
        })?;

    Ok(Html(result))
}

pub fn render_404(message: &str) -> poem::Result<Html<String>> {
    let context = context! {
        message => message,
        ..default_context(TemplateEnv::default())
    };

    render_template("errors/404.html", context)
}
