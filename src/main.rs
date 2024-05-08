/**
 * Copyright 2024-present iamyunsin
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

#[macro_use]
extern crate rocket;

use rocket::http::Status;
use rocket_db_pools::{Connection, Database};
use sinsuan::lib::cors::CORS;
use uuid::Uuid;
use std::fmt::Debug;
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
      let count_url =  request.headers().get_one("X-Sinsuan-Count-Url");

      // 优先从cookie中获取用户唯一标识
      let mut user_id = match request.cookies().get("sinsuanid") {
        Some(uid) => uid.value().to_string(),
        None => "".to_string(),
      };

      // 如果cookie中没有用户唯一标识，再从请求头中获取
      if user_id.is_empty() {
        user_id =  match request.headers().get_one("X-Sinsuan-Id") {
          Some(uid) => uid.to_string(),
          None => "".to_string(),
        };
      }

      if count_url.is_none() {
        return Outcome::Success(SinSuanDto {
          domain: None,
          path: None,
          user_id: None,
        });
      }

      let url = Url::parse(count_url.unwrap()).unwrap();

      Outcome::Success(SinSuanDto {
        domain: url.host_str().map(|s| s.to_string()),
        path: url.path().to_string().parse().ok(),
        // cookie和请求头中都没有用户唯一标识，生成一个
        user_id: if user_id.is_empty() { Some(Uuid::now_v7().to_string()) }  else { Some(user_id) },
      })
    }
}

#[get("/count")]
async fn count(sin_suan_dto: SinSuanDto, mut db: Connection<SinSuanDB>) -> Option<CombinedVisitCount> {
  // 如果有参数为空，不进行后续操作
  if sin_suan_dto.domain.is_none() || sin_suan_dto.path.is_none() || sin_suan_dto.user_id.is_none() {
    return None;
  }

  let domain = sin_suan_dto.domain.unwrap();
  let path = sin_suan_dto.path.unwrap();
  let user_id = sin_suan_dto.user_id.unwrap();

  // 首次访问可能需要创建表、视图和物化视图触发器
  let _ = storage::init_domain_storage(&mut db, domain.clone()).await;

  // 记录本次访问
  let _ = storage::record_visit(&mut db, domain.clone(), VisitRecord {
    path: path.clone(),
    user_id: user_id.clone(),
  }).await;

  // 查询统计数据
  let mut counts = storage::query_count(&mut db, domain.as_str(), path).await.unwrap();
  counts.sin_suan_id = Some(user_id);

  Some(counts)
}

#[options("/count")]
async fn count_cors() -> Status {
  Status::Ok
}

#[launch]
async fn rocket() -> _ {
  rocket::build()
    .attach(CORS)
    .attach(SinSuanDB::init())
    .mount("/", routes![count, count_cors])
}
