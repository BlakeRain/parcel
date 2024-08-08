use poem::{handler, web::Data, IntoResponse};

use crate::{
    app::templates::{authorized_context, render_template},
    env::Env,
    model::user::User,
};

#[handler]
pub fn get_index(env: Data<&Env>, user: User) -> poem::Result<impl IntoResponse> {
    let context = authorized_context(&env, &user);
    render_template("index.html", context)
}
