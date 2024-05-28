use rocket::{http::Status, State};
use rocket_db_pools::Connection;
use uuid::Uuid;
use std::{fmt::Debug, net::IpAddr};
use rocket::request::{Outcome, FromRequest, Request};
use url::Url;
use crate::utils::storage::{self, CombinedVisitCount, SinSuanDB, VisitRecord};
use crate::app_config::Config;

/** 心算请求数据结构 */
#[derive(Debug)]
pub struct SinSuanDto {
  /** 统计域名 */
  domain: Option<String>,
  /** 统计路径 */
  path: Option<String>,
  /** 用户唯一标识，客户端可以主动设置，若未设置，则表示 */
  user_id: Option<String>,
  /** 访问者ip */
  ip: Option<IpAddr>,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for SinSuanDto {
    type Error = std::convert::Infallible;

    async fn from_request(request: &'r Request<'_>) -> Outcome<SinSuanDto, Self::Error> {
      let count_url =  request.headers().get_one("X-Sinsuan-Count-Url");
      let user_id = request.headers().get_one("X-Sinsuan-Id").unwrap_or("").to_string();
      let ip = request.client_ip();

      if count_url.is_none() {
        return Outcome::Success(SinSuanDto {
          domain: None,
          path: None,
          user_id: None,
          ip: None,
        });
      }

      let url = Url::parse(count_url.unwrap()).unwrap();

      Outcome::Success(SinSuanDto {
        domain: url.host_str().map(|s| s.to_string()),
        path: url.path().to_string().parse().ok(),
        // cookie和请求头中都没有用户唯一标识，生成一个
        user_id: if user_id.is_empty() { Some(Uuid::now_v7().to_string()) }  else { Some(user_id) },
        ip,
      })
    }
}

#[get("/count")]
pub async fn get_count(sin_suan_dto: SinSuanDto, mut db: Connection<SinSuanDB>, config: &State<Config>) -> Option<CombinedVisitCount> {
  // 如果有参数为空，不进行后续操作
  if sin_suan_dto.domain.is_none() || sin_suan_dto.path.is_none() || sin_suan_dto.user_id.is_none() {
    return None;
  }

  let domain = sin_suan_dto.domain.unwrap();
  let path = sin_suan_dto.path.unwrap();
  let user_id = sin_suan_dto.user_id.unwrap();
  let ip = sin_suan_dto.ip.unwrap_or(IpAddr::from([127, 0, 0, 1]));

  // 首次访问可能需要创建表、视图和物化视图触发器
  let _ = storage::init_domain_storage(&mut db, domain.clone()).await;

  // 更新访问者地理位置信息
  storage::update_location(&mut db, ip.clone().to_string(), config.inner()).await;

  // 记录本次访问
  let _ = storage::record_visit(&mut db, domain.clone(), VisitRecord {
    path: path.clone(),
    user_id: user_id.clone(),
    ip: ip.to_string(),
  }).await;

  // 查询统计数据
  let mut counts = storage::query_count(&mut db, domain.as_str(), path).await.unwrap();
  counts.sin_suan_id = Some(user_id);

  Some(counts)
}

#[options("/count")]
pub async fn count_cors() -> Status {
  Status::Ok
}
