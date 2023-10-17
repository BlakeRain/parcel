use poem::{handler, IntoResponse};

use crate::{
    app::templates::{authorized_context, render_template},
    model::user::User,
};

#[handler]
pub async fn get_index(user: User) -> poem::Result<impl IntoResponse> {
    let context = authorized_context(&user);
    render_template("index.html", &context)
}
