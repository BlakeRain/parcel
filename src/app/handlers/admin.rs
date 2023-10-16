use poem::{handler, web::Html};

use crate::app::{
    extractors::admin::Admin,
    templates::{authorized_context, render_template},
};

pub mod setup;
pub mod uploads;
pub mod users;

#[handler]
pub fn get_admin(Admin(admin): Admin) -> poem::Result<Html<String>> {
    let context = authorized_context(&admin);
    render_template("admin.html", &context)
}
