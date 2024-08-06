use std::collections::HashMap;

use lazy_static::lazy_static;
use poem::error::InternalServerError;
use poem::web::Html;
use tera::Context;
use tera::Tera;
use time::macros::format_description;
use time::Date;
use time::OffsetDateTime;
use time::Time;

use crate::env::Env;
use crate::model::user::User;

lazy_static! {
    pub static ref TEMPLATES: Tera = {
        let mut tera = Tera::new("templates/**/*").expect("to load templates");
        tera.autoescape_on(vec![".html", ".svg"]);

        tera.register_filter(
            "datetime",
            |value: &tera::Value, _: &HashMap<String, tera::Value>| -> tera::Result<tera::Value> {
                let time = serde_json::from_value::<OffsetDateTime>(value.clone())?;
                Ok(tera::to_value(
                    time.format(format_description!(
                        "[year]-[month]-[day] [hour]:[minute]:[second]"
                    ))
                    .expect("formatted time"),
                )?)
            },
        );

        tera.register_filter(
            "datetime_offset",
            |value: &tera::Value, _: &HashMap<String, tera::Value>| -> tera::Result<tera::Value> {
                let time = serde_json::from_value::<OffsetDateTime>(value.clone())?;
                let offset = time - OffsetDateTime::now_utc();
                Ok(tera::to_value(
                    time_humanize::HumanTime::from(offset.whole_seconds()).to_string(),
                )?)
            },
        );

        tera.register_filter(
            "date",
            |value: &tera::Value, _: &HashMap<String, tera::Value>| -> tera::Result<tera::Value> {
                let time = serde_json::from_value::<Date>(value.clone())?;
                Ok(tera::to_value(
                    time.format(format_description!("[year]-[month]-[day]"))
                        .expect("formatted time"),
                )?)
            },
        );

        tera.register_filter(
            "date_offset",
            |value: &tera::Value, _: &HashMap<String, tera::Value>| -> tera::Result<tera::Value> {
                let time = serde_json::from_value::<Date>(value.clone())?;
                let offset = time - time::OffsetDateTime::now_utc().date();
                Ok(tera::to_value(
                    time_humanize::HumanTime::from(offset.whole_seconds()).to_string(),
                )?)
            },
        );

        tera.register_filter(
            "time",
            |value: &tera::Value, _: &HashMap<String, tera::Value>| -> tera::Result<tera::Value> {
                let time = serde_json::from_value::<Time>(value.clone())?;
                Ok(tera::to_value(
                    time.format(format_description!("[hour]:[minute]:[second]"))
                        .expect("formatted time"),
                )?)
            },
        );

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

#[derive(Default)]
pub struct TemplateEnv<'a> {
    pub analytics_domain: Option<&'a str>,
    pub plausible_script: Option<&'a str>,
}

impl<'a> From<&'a Env> for TemplateEnv<'a> {
    fn from(env: &'a Env) -> Self {
        Self {
            analytics_domain: env.analytics_domain.as_deref(),
            plausible_script: env.plausible_script.as_deref(),
        }
    }
}

impl<'a> From<&poem::web::Data<&'a Env>> for TemplateEnv<'a> {
    fn from(env: &poem::web::Data<&'a Env>) -> Self {
        Self {
            analytics_domain: env.analytics_domain.as_deref(),
            plausible_script: env.plausible_script.as_deref(),
        }
    }
}

pub fn default_context<'e, E: Into<TemplateEnv<'e>>>(env: E) -> Context {
    let env = env.into();
    let mut context = BASE_CONTEXT.clone();

    context.insert(
        "env",
        &serde_json::json!({
            "analytics_domain": &env.analytics_domain,
            "plausible_script": &env.plausible_script,
        }),
    );

    context
}

pub fn authorized_context<'e, E: Into<TemplateEnv<'e>>>(env: E, user: &User) -> Context {
    let mut context = default_context(env);
    context.insert("user", user);
    context
}

pub fn render_template(name: &str, context: &Context) -> poem::Result<Html<String>> {
    TEMPLATES
        .render(name, context)
        .map_err(|err| {
            tracing::error!(template_name = ?name, "Template render failed: {err:?}");
            InternalServerError(err)
        })
        .map(Html)
}

pub fn render_404(message: &str) -> poem::Result<Html<String>> {
    let mut context = default_context(TemplateEnv::default());
    context.insert("message", message);
    render_template("errors/404.html", &context)
}
