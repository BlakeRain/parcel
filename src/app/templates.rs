use lazy_static::lazy_static;
use poem::error::InternalServerError;
use poem::web::Html;
use tera::Context;
use tera::Tera;

use crate::model::user::User;

lazy_static! {
    pub static ref TEMPLATES: Tera = {
        let mut tera = Tera::new("templates/**/*").expect("to load templates");
        tera.autoescape_on(vec![".html"]);
        tera
    };
    pub static ref BASE_CONTEXT: Context = {
        let mut context = Context::new();

        context.insert(
            "build",
            &serde_json::json!({
                "version": env!("CARGO_PKG_VERSION"),
                "date": env!("CARGO_BUILD_DATE"),
                "git": {
                    "commit": env!("CARGO_GIT_COMMIT"),
                    "short": env!("CARGO_GIT_SHORT"),
                },
            }),
        );

        context
    };
}

pub fn default_context() -> Context {
    BASE_CONTEXT.clone()
}

pub fn authorized_context(user: &User) -> Context {
    let mut context = default_context();
    context.insert("user", user);
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

pub fn render_404(message: &str) -> poem::Result<Html<String>> {
    let mut context = default_context();
    context.insert("message", message);
    render_template("errors/404.html", &context)
}
