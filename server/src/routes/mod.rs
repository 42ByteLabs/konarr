use rocket::{fs::NamedFile, Request};

#[catch(404)]
pub async fn failed_not_found<'r>(_req: &'r Request<'_>) -> NamedFile {
    // TODO: Static page
    NamedFile::open("dist/index.html").await.unwrap()
}

pub fn routes() -> Vec<rocket::Route> {
    routes![]
}
