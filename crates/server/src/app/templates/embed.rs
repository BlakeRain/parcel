use std::sync::OnceLock;

use minijinja::Environment;
use rust_embed::RustEmbed;

use super::functions::add_to_environment;

#[derive(RustEmbed)]
#[folder = "$CARGO_MANIFEST_DIR/templates/"]
struct TemplatesEmbed;

fn make_templates<E: RustEmbed>() -> Environment<'static> {
    let mut environment = Environment::new();

    tracing::info!("Loading embedded templates");
    for path in E::iter() {
        let Some(file) = E::get(&path) else {
            tracing::error!(?path, "Failed to load embedded template");
            continue;
        };

        let content = match std::str::from_utf8(file.data.as_ref()) {
            Ok(content) => content,
            Err(err) => {
                tracing::error!(?path, error = ?err, "Failed to decode embedded template as UTF-8");
                continue;
            }
        };

        if let Err(err) = environment.add_template_owned(path.clone(), content.to_owned()) {
            tracing::error!(?path, error = %err, "Failed to load embedded template")
        }
    }

    add_to_environment(&mut environment);
    environment
}

pub async fn get_templates() -> &'static Environment<'static> {
    static TEMPLATES: OnceLock<Environment<'static>> = OnceLock::new();
    TEMPLATES.get_or_init(|| make_templates::<TemplatesEmbed>())
}
