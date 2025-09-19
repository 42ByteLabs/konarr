use rocket::State;
use rocket::fs::NamedFile;

use crate::AppState;

#[get("/<_..>", rank = 20, format = "text/html")]
pub async fn index(state: &State<AppState>) -> NamedFile {
    let path = state.config.frontend_path().unwrap().join("index.html");
    NamedFile::open(path).await.unwrap()
}

pub fn routes() -> Vec<rocket::Route> {
    routes![index]
}
