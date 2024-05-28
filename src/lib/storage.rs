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

use lazy_static::lazy_static;
use log;

use rocket::http::{ContentType, Status};
use rocket::serde::{json, Deserialize, Serialize};
use rocket::{response, Request, Response};
use rocket_db_pools::{sqlx, Database, Connection};
use regex::Regex;
use md5::{Md5, Digest};
use hex;
use urlencoding::encode;

use crate::rocket;
use crate::app_config;

/** 数据库连接池 */
#[derive(Database)]
#[database("sin_suan")]
pub struct SinSuanDB(sqlx::SqlitePool);

/** 用户访问记录 */
pub struct VisitRecord {
  /** 访问路径 */
  pub path: String,
  /** 用户唯一标识 */
  pub user_id: String,
  /** 访问ip，可用于构造访问者地图 */
  pub ip: String,
}

#[derive(sqlx::FromRow, Debug)]
/** 访问统计模型 */
pub struct VisitCount {
  pub pv: u32,
  pub uv: u32,
}


#[derive(sqlx::FromRow, Debug)]
/** 访问统计模型 */
pub struct SiteVisitCount {
  pub site_pv: u32,
  pub site_uv: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CombinedVisitCount {
  pub sin_suan_id: Option<String>,
  pub pv: u32,
  pub uv: u32,
  pub site_pv: u32,
  pub site_uv: u32,
}

#[derive(sqlx::FromRow, Debug)]
/** 访问统计模型 */
pub struct PageVisitCount {
  pub pv: u32,
  pub uv: u32,
  pub site_pv: u32,
  pub site_uv: u32,
}


impl<'r, 'o> rocket::response::Responder<'r, 'static> for CombinedVisitCount {
  fn respond_to(self, req: &'r Request) -> response::Result<'static> {
    Response::build_from(json::to_string(&self).unwrap().respond_to(req)?)
        .status(Status::Ok)
        .header(ContentType::JSON)
        .ok()
  }
}

lazy_static! {
  static ref TABLE_NAME_REPLACER: Regex = Regex::new(
      r"[^a-zA-Z0-9_]"
      ).unwrap();
}

fn get_table_name(prefix: &str, domain: &str) -> String {
  TABLE_NAME_REPLACER.replace_all(&format!("{}_{}", prefix, domain), "_").to_string()
}

/** 站点访问相关的表是否已经完成初始化 */
async fn is_domain_table_ready(db: &mut Connection<SinSuanDB>, domain: String) -> Result<bool, sqlx::Error> {
  // let executor = &mut **db;
let table_exists = sqlx::query(
      "SELECT name FROM sqlite_master WHERE type='table' AND name=$1 LIMIT 1")
    .bind(get_table_name("visit_record", domain.as_str()))
    .fetch_optional(db.as_mut())
    .await?;
  Ok(table_exists.is_some())
}

/** 创建访问记录表 */
async fn create_visit_record_table(db: &mut Connection<SinSuanDB>, domain: &str) -> Result<(), sqlx::Error> {
  sqlx::query(
  &format!(r#"
    CREATE TABLE IF NOT EXISTS {} (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      path TEXT NOT NULL,
      user_id TEXT NOT NULL,
      timestamp TEXT NOT NULL
    )
    "#,
    get_table_name("visit_record", domain)
  ))
  .execute(db.as_mut())
  .await?;
  Ok(())
}

#[derive(sqlx::FromRow, Debug)]
struct TableColumn {
  // cid: String,
  name: String,
  // #[sqlx(rename = "type")]
  // col_type: String,
  // notnull: bool,
  // dflt_value: Option<String>,
  // pk: i32,
}

async fn alter_visit_record_table(db: &mut Connection<SinSuanDB>, domain: &str) -> Result<(), sqlx::Error> {

  let columns = sqlx::query_as::<_, TableColumn>(
    &format!(r#"
      PRAGMA table_info({})
    "#,
    get_table_name("visit_record", domain)
  ))
  .fetch_all(db.as_mut())
  .await?;

  // 如果存在ip字段，说明已经升级完成了
  for column in columns {
    log::info!("column: {:?}", column);
    if column.name == "ip" {
      return Ok(());
    }
  }

  // 执行当前域名数据库升级脚本：
  // 1. 添加ip字段
  // 2. 删除触发器
  // 3. 删除物理视图
  sqlx::query(
    &format!(r#"
      ALTER TABLE {} ADD COLUMN ip TEXT DEFAULT NULL;
      DROP TRIGGER IF EXISTS {};
      DROP TABLE IF EXISTS {};

      CREATE TABLE IF NOT EXISTS `ip_location` (
        ip TEXT NOT NULL PRIMARY KEY,
        nation TEXT DEFAULT NULL,
        province TEXT DEFAULT NULL,
        city TEXT DEFAULT NULL,
        district TEXT DEFAULT NULL,
        lat REAL DEFAULT NULL,
        lon REAL DEFAULT NULL
      );

      CREATE UNIQUE INDEX ip_location_unique on `ip_location` (`ip`);
      CREATE INDEX ip_location_notion on `ip_location` (`nation`);
    "#,
    get_table_name("visit_record", domain),
    get_table_name("visit_count_view_trigger", domain),
    get_table_name("visit_count_view", domain),
  ))
  .execute(db.as_mut())
  .await?;
  Ok(())
}

pub async fn init_domain_storage(db: &mut Connection<SinSuanDB>, domain: String) -> Result<(), sqlx::Error> {
  if is_domain_table_ready(db, domain.clone()).await? {
    alter_visit_record_table(db, &domain).await?;
    return Ok(());
  }
  create_visit_record_table(db, domain.clone().as_str()).await?;
  Ok(())
}

pub async fn query_count(db: &mut Connection<SinSuanDB>, domain: &str, path: String) -> Result<CombinedVisitCount, sqlx::Error> {
  let page_visit_count = sqlx::query_as::<_, PageVisitCount>(
    &format!(
      r#"
        SELECT
          COUNT(*) AS site_pv,
          COUNT(DISTINCT user_id) as site_uv,
          COUNT(
            CASE WHEN path = $1 THEN 1 ELSE NULL END
          ) as pv,
          COUNT(DISTINCT CASE WHEN path = $1 THEN user_id ELSE NULL END) as uv
          FROM {}
      "#,
      get_table_name("visit_record", domain)
    )
  )
  .bind(path)
  .fetch_one(db.as_mut())
  .await?;

  Ok(CombinedVisitCount {
    sin_suan_id: None,
    pv: page_visit_count.pv,
    uv: page_visit_count.uv,
    site_pv: page_visit_count.site_pv,
    site_uv: page_visit_count.site_uv,
  })
}

/** 更新访问者的地理位置信息 */
pub async fn update_location(db: &mut Connection<SinSuanDB>, ip: String, config: &app_config::Config) {
  let map_config = config.qq_map.clone();

  // 如果没有配置地图key和sk，则跳过
  if map_config.key.is_empty() || map_config.sk.is_empty() {
    return;
  }

  log::info!("update location: {}", ip);

  // 如果是本地回环地址，直接忽略
  if ip == "127.0.0.1" {
    return;
  }

  let ip_exists = sqlx::query(
    "SELECT province FROM ip_location WHERE ip = $1")
  .bind(ip.clone())
  .fetch_optional(db.as_mut())
  .await.unwrap();
  // 如果本地存在ip地址的地理位置信息，则直接不再请求腾讯地图api
  if ip_exists.is_some() {
    return;
  }

  // 调用腾讯地图api，通过ip地址获取地理位置信息
  let req_path = format!("/ws/location/v1/ip?key={}&ip={}", encode(map_config.key.as_str()), encode(ip.as_str()));
  let hash_path = format!("/ws/location/v1/ip?ip={}&key={}{}", encode(ip.as_str()), encode(map_config.key.as_str()), encode(map_config.sk.as_str()));
  let mut hasher = Md5::new();
  hasher.update(hash_path);
  let sig = hasher.finalize();
  let sig_hex = hex::encode(sig);
  let req_url = format!("{}{}&sig={}", map_config.base_url, req_path, encode(sig_hex.as_str()));


  let res = reqwest::get(req_url)
    .await.unwrap()
    .json::<serde_json::Value>()
    .await.unwrap();

  if let Some(result) = res.get("result") {
    let ad_info = result.get("ad_info").unwrap();
    let location = result.get("location").unwrap();
    let result = sqlx::query(r#"
      INSERT INTO
        `ip_location`
        (ip, nation, province, city, district, lat, lon)
        VALUES
        ($1, $2, $3, $4, $5, $6, $7)"#)
      .bind(ip)
      .bind(ad_info.get("nation").unwrap().as_str().unwrap())
      .bind(ad_info.get("province").unwrap().as_str().unwrap())
      .bind(ad_info.get("city").unwrap().as_str().unwrap())
      .bind(ad_info.get("district").unwrap().as_str().unwrap())
      .bind(location.get("lat").unwrap().as_f64().unwrap())
      .bind(location.get("lng").unwrap().as_f64().unwrap())
      .execute(db.as_mut())
      .await.unwrap();

    log::info!("New ip location record {:?}", result);
  }
}

pub async fn record_visit(db: &mut Connection<SinSuanDB>, domain: String, visit_record: VisitRecord) -> Result<(), sqlx::Error> {
  let domain_table_name = get_table_name("visit_record", domain.as_str());

  // 插入访问记录
  let result = sqlx::query(
    &format!(
        "INSERT INTO {} (path, user_id, ip, timestamp) VALUES ($1, $2, $3, CURRENT_TIMESTAMP)",
        domain_table_name
      )
    )
    .bind(visit_record.path)
    .bind(visit_record.user_id)
    .bind(visit_record.ip.clone())
    .execute(db.as_mut())
    .await.unwrap();

  log::info!("New visit record {:?}", result);
  Ok(())
}