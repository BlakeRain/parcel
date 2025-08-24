use poem::{error::InternalServerError, web::Html};
use serde::Serialize;

pub mod context;
pub mod functions;

#[cfg(debug_assertions)]
pub mod tailwind;

#[cfg(debug_assertions)]
pub mod reload;

#[cfg(not(debug_assertions))]
pub mod embed;

pub use context::*;

#[cfg(debug_assertions)]
use reload::get_templates;

#[cfg(not(debug_assertions))]
use embed::get_templates;

pub async fn render_template<S: Serialize>(name: &str, context: S) -> poem::Result<Html<String>> {
    let result = get_templates()
        .await
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
