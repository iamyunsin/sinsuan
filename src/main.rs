#[macro_use]
extern crate rocket;

use rocket_db_pools::{Connection, Database};
use std::fmt::Debug;

use rocket::http::hyper::header;
use rocket::request::{Outcome, FromRequest, Request};
use url::Url;

use sinsuan::lib::storage::{self, CombinedVisitCount, SinSuanDB, VisitRecord};

/** 心算请求数据结构 */
#[derive(Debug)]
struct SinSuanDto {
  /** 统计域名 */
  domain: Option<String>,
  /** 统计路径 */
  path: Option<String>,
  /** 用户唯一标识，客户端可以主动设置，若未设置，则表示 */
  user_id: Option<String>,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for SinSuanDto {
    type Error = std::convert::Infallible;

    async fn from_request(request: &'r Request<'_>) -> Outcome<SinSuanDto, Self::Error> {
      let referer =  request.headers().get_one(header::REFERER.as_str());

      let user_id: String =  match request.headers().get_one("X-SinSuan-ID") {
        Some(uid) => uid.to_string(),
        None => "".to_string(),
      };

      let url = Url::parse(referer.unwrap_or("https://yunsin.top/abd")).unwrap();

      Outcome::Success(SinSuanDto {
        domain: url.host_str().map(|s| s.to_string()),
        path: url.path().to_string().parse().ok(),
        user_id: if user_id.is_empty() { None }  else { Some(user_id) },
      })
    }
}

#[get("/count")]
async fn view(sin_suan_dto: SinSuanDto, mut db: Connection<SinSuanDB>) -> Option<CombinedVisitCount> {
  let domain = sin_suan_dto.domain.unwrap();
  let path = sin_suan_dto.path.unwrap();
  let user_id = sin_suan_dto.user_id.unwrap();

  // 首次访问可能需要创建表
  let _ = storage::create_domain_tables(&mut db, domain.clone()).await;

  // 记录本次访问
  let _ = storage::record_visit(&mut db, domain.clone(), VisitRecord {
    path: path.clone(),
    user_id: user_id.clone(),
  }).await;

  // 返回统计数据
  Some(storage::query_count(&mut db, domain.as_str(), path).await.unwrap())
}

#[launch]
fn rocket() -> _ {
  rocket::build()
    .attach(SinSuanDB::init())
    .mount("/", routes![view])
}
