use poem::{handler, web::Redirect};

#[handler]
pub fn get_index() -> poem::Result<Redirect> {
    Ok(Redirect::see_other("/uploads"))
}
