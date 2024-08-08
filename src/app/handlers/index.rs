use poem::{
    handler,
    web::{Data, Query},
    IntoResponse,
};

use crate::{
    app::templates::{authorized_context, render_template},
    env::Env,
    model::user::User,
};

use super::uploads::ListSorting;

#[handler]
pub fn get_index(
    env: Data<&Env>,
    user: User,
    Query(sorting): Query<ListSorting>,
) -> poem::Result<impl IntoResponse> {
    render_template(
        "index.html",
        minijinja::context! {
            sorting,
            ..authorized_context(&env, &user)
        },
    )
}
