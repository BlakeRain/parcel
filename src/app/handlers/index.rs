use poem::{handler, web::Html};

use crate::app::templates::{default_context, render_template};

#[handler]
pub fn get_index() -> poem::Result<Html<String>> {
    let context = default_context();
    render_template("index.html", &context)
}
