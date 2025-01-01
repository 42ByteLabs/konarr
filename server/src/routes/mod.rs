use rocket::fs::NamedFile;

#[get("/<_..>", rank = 20, format = "text/html")]
pub async fn index() -> NamedFile {
    NamedFile::open("dist/index.html").await.unwrap()
}

pub fn routes() -> Vec<rocket::Route> {
    routes![index]
}
