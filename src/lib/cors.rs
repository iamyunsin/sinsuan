use rocket::{fairing::{Fairing, Info, Kind}, http::hyper::header::ORIGIN};
pub struct CORS;

#[rocket::async_trait]
impl Fairing for CORS {
    fn info(&self) -> Info {
        Info {
            name: "Add CORS headers to responses",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, req: &'r rocket::Request<'_>, res: &mut rocket::Response<'r>) {
        res.set_header(rocket::http::Header::new("Access-Control-Allow-Origin", req.headers().get_one(ORIGIN.as_str()).unwrap_or("*")));
        res.set_header(rocket::http::Header::new("Access-Control-Allow-Methods", "GET, OPTIONS"));
        res.set_header(rocket::http::Header::new("Access-Control-Allow-Headers", "Content-Type,X-Sinsuan-Count-Url,X-Sinsuan-Id"));
        res.set_header(rocket::http::Header::new("Access-Control-Allow-Credentials", "true"));
    }
}