use esbuild_bundle::javascript;
use minijinja::{context, Value};

use parcel_model::user::User;

use crate::env::Env;

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

fn create_build_info() -> Value {
    context! {
        profile => env!("CARGO_PROFILE"),
        version => env!("CARGO_PKG_VERSION"),
        date => env!("CARGO_BUILD_DATE")
    }
}

pub fn default_context<'e, E: Into<TemplateEnv<'e>>>(env: E) -> Value {
    let env = env.into();

    context! {
        build => create_build_info(),
        env => context! {
            init_js => javascript!("$CARGO_MANIFEST_DIR/scripts/init.ts"),
            analytics_domain => env.analytics_domain,
            plausible_script => env.plausible_script,
        }
    }
}

pub fn authorized_context<'e, E: Into<TemplateEnv<'e>>>(env: E, user: &User) -> Value {
    context! {
        auth => context! {
            has_totp => user.totp.is_some(),
            ..Value::from_serialize(user)
        },
        ..default_context(env)
    }
}
