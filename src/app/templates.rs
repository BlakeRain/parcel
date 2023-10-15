use lazy_static::lazy_static;
use poem::error::InternalServerError;
use poem::web::Html;
use tera::Context;
use tera::Tera;

lazy_static! {
    pub static ref TEMPLATES: Tera = {
        let mut tera = Tera::new("templates/**/*").expect("to load templates");
        tera.autoescape_on(vec![".html"]);
        tera
    };
}

pub fn default_context() -> Context {
    let mut context = Context::new();

    context.insert("version", env!("CARGO_PKG_VERSION"));
    context.insert("build_date", env!("CARGO_BUILD_DATE"));
    context.insert("git_commit", env!("CARGO_GIT_COMMIT"));

    context
}

pub fn render_template(name: &str, context: &Context) -> poem::Result<Html<String>> {
    TEMPLATES
        .render(name, context)
        .map_err(|err| {
            tracing::error!("render '{name}' failed: {err:?}");
            InternalServerError(err)
        })
        .map(Html)
}
