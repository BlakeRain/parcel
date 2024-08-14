use minijinja::context;
use poem::{
    error::InternalServerError,
    handler,
    http::StatusCode,
    web::{Data, Html, Path, Query, Redirect},
    IntoResponse, Response,
};
use serde::Deserialize;

use crate::{
    app::templates::{authorized_context, render_404, render_template},
    env::Env,
    model::{
        upload::{Upload, UploadStats},
        user::User,
    },
};

mod download;
mod edit;
mod list;
mod new;
mod upload;

pub use download::{get_download, post_download};
pub use edit::{get_edit, post_check_slug, post_edit};
pub use list::{get_list, get_page, post_delete, ListQuery};
pub use new::{get_new, post_new};
pub use upload::{delete_upload, get_custom_upload, get_upload};

#[handler]
pub async fn get_stats(env: Data<&Env>, user: User) -> poem::Result<Html<String>> {
    let stats = UploadStats::get_for(&env.pool, user.id)
        .await
        .map_err(InternalServerError)?;

    // Generate a random int between 10 and 90
    let random = rand::random::<i8>() % 80 + 10;

    render_template(
        "uploads/stats.html",
        context! {
            stats,
            random,
            ..authorized_context(&env, &user)
        },
    )
}

#[derive(Debug, Deserialize)]
pub struct MakePublicQuery {
    public: bool,
    ult_dest: Option<String>,
}

#[handler]
pub async fn post_public(
    env: Data<&Env>,
    user: User,
    Path(id): Path<i32>,
    Query(MakePublicQuery { public, ult_dest }): Query<MakePublicQuery>,
) -> poem::Result<Response> {
    let Some(mut upload) = Upload::get(&env.pool, id).await.map_err(|err| {
        tracing::error!(err = ?err, id = ?id, "Unable to get upload by ID");
        InternalServerError(err)
    })?
    else {
        tracing::error!("Unrecognized upload ID '{id}'");
        return render_404("Unrecognized upload ID").map(IntoResponse::into_response);
    };

    if !user.admin && upload.uploaded_by != user.id {
        tracing::error!(
            user = user.id,
            upload = upload.id,
            "User tried to edit upload without permission"
        );

        return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
    }

    tracing::info!(id = id, public = public, "Setting upload public state");
    upload
        .set_public(&env.pool, public)
        .await
        .map_err(InternalServerError)?;

    if let Some(ult_dest) = ult_dest {
        Ok(Redirect::see_other(ult_dest).into_response())
    } else {
        Ok(Html("").with_header("HX-Redirect", "/").into_response())
    }
}

#[handler]
pub async fn post_reset(
    env: Data<&Env>,
    user: User,
    Path(id): Path<i32>,
) -> poem::Result<Html<String>> {
    let Some(mut upload) = Upload::get(&env.pool, id).await.map_err(|err| {
        tracing::error!(err = ?err, id = ?id, "Unable to get upload by ID");
        InternalServerError(err)
    })?
    else {
        tracing::error!("Unrecognized upload ID '{id}'");
        return render_404("Unrecognized upload ID");
    };

    if !user.admin && upload.uploaded_by != user.id {
        tracing::error!(
            user = user.id,
            upload = upload.id,
            "User tried to reset upload without permission"
        );
        return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
    }

    tracing::info!(id = id, "Resetting upload download stats");
    upload
        .reset_remaining(&env.pool)
        .await
        .map_err(InternalServerError)?;

    Ok(Html(if let Some(limit) = upload.limit {
        format!(
            "<td class=\"text-right text-nowrap\">{} / {}</td>",
            limit, limit
        )
    } else {
        "<i>Unlimited</i>".to_string()
    }))
}
