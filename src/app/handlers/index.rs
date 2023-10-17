use poem::{error::InternalServerError, handler, web::Data, IntoResponse};

use crate::{
    app::templates::{authorized_context, render_template},
    env::Env,
    model::{upload::Upload, user::User},
};

#[handler]
pub async fn get_index(env: Data<&Env>, user: User) -> poem::Result<impl IntoResponse> {
    let uploads = Upload::get_for_user(&env.pool, user.id)
        .await
        .map_err(InternalServerError)?;

    let mut context = authorized_context(&user);
    context.insert("uploads", &uploads);
    render_template("index.html", &context)
}
