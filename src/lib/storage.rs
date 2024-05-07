use lazy_static::lazy_static;

use rocket::http::{ContentType, Status};
use rocket::serde::{json, Deserialize, Serialize};
use rocket::{response, Request, Response};
use rocket_db_pools::{sqlx, Database, Connection};
use regex::Regex;


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

#[derive(Serialize, Deserialize, Debug)]
pub struct CombinedVisitCount {
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
pub async fn is_domain_table_ready(db: &mut Connection<SinSuanDB>, domain: String) -> Result<bool, sqlx::Error> {
  // let executor = &mut **db;
let table_exists = sqlx::query(
      "SELECT name FROM sqlite_master WHERE type='table' AND name=$1 LIMIT 1")
    .bind(get_table_name("visit_record", domain.as_str()))
    .fetch_optional(db.as_mut())
    .await?;
  Ok(table_exists.is_some())
}

/** 创建访问记录表 */
pub async fn create_visit_record_table(db: &mut Connection<SinSuanDB>, domain: &str) -> Result<(), sqlx::Error> {
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

/** 创建访问记录统计物化视图 */
pub async fn create_visit_count_materialized_view(db: &mut Connection<SinSuanDB>, domain: &str) -> Result<(), sqlx::Error> {
  let record_table_name = get_table_name("visit_record", domain);
  let count_materialized_view_name = get_table_name("visit_count_view", domain);
  let materialized_view_trigger = get_table_name("visit_count_view_trigger", domain);
  // 创建统计物化视图
  sqlx::query(
    &format!(
      r#"
      CREATE TABLE IF NOT EXISTS {} (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        path TEXT NOT NULL,
        uv INTEGER NOT NULL,
        pv INTEGER NOT NULL
      )
      "#,
      count_materialized_view_name.clone()
    )
  )
  .execute(db.as_mut())
  .await?;
  // 创建触发器
  sqlx::query(
    &format!(
      r#"
        CREATE TRIGGER {}
        AFTER INSERT ON {}
        BEGIN
          DELETE FROM {} WHERE path = NEW.path;
          INSERT INTO {} (path, uv, pv) select path, COUNT(DISTINCT user_id), COUNT(*) FROM {} WHERE path = NEW.path;
        END;
      "#,
      materialized_view_trigger,
      record_table_name.clone(),
      count_materialized_view_name.clone(),
      count_materialized_view_name.clone(),
      record_table_name.clone()
    )
  )
  .execute(db.as_mut())
  .await?;

  Ok(())
}

pub async fn create_domain_tables(db: &mut Connection<SinSuanDB>, domain: String) -> Result<(), sqlx::Error> {
  if is_domain_table_ready(db, domain.clone()).await? {
    return Ok(());
  }
  create_visit_record_table(db, domain.clone().as_str()).await?;
  create_visit_count_materialized_view(db, domain.clone().as_str()).await?;
  Ok(())
}

/** 根据域名查询站点访问数据 */
async fn query_site_count(db: &mut Connection<SinSuanDB>, domain: &str) -> Result<SiteVisitCount, sqlx::Error> {
  let site_visit_count = sqlx::query_as::<_, SiteVisitCount>(
  &format!(
        "SELECT SUM(pv) as site_pv, SUM(uv) as site_uv FROM {}",
        get_table_name("visit_count_view", domain)
      )
    )
    .fetch_one(db.as_mut())
    .await?;

  Ok(site_visit_count)
}

/** 查询站点下某个页面的访问记录 */
async fn query_page_count(db: &mut Connection<SinSuanDB>, domain: &str, path: String) -> Result<VisitCount, sqlx::Error> {
  let count_materialized_view_name = get_table_name("visit_count_view", domain);
  // 已经初始化过了
  let page_visit_count = sqlx::query_as::<_, VisitCount>(
      &format!(
        "SELECT path, uv, pv FROM {} WHERE path = $1",
        count_materialized_view_name
      )
    )
    .bind(path)
    .fetch_one(db.as_mut())
    .await?;

  Ok(page_visit_count)
}

pub async fn query_count(db: &mut Connection<SinSuanDB>, domain: &str, path: String) -> Result<CombinedVisitCount, sqlx::Error> {
  // let arcDB = Arc::new(Mutex::new(db));
  let page_count_task = query_page_count(db, domain, path.clone()).await;
  let site_count_task = query_site_count(db, domain).await;

  // let (page_count, site_count) = tokio::join!(page_count_task, site_count_task);

  let page_count = page_count_task.unwrap();
  let site_count = site_count_task.unwrap();

  Ok(CombinedVisitCount {
    pv: page_count.pv,
    uv: page_count.uv,
    site_pv: site_count.site_pv,
    site_uv: site_count.site_uv,
  })
}


pub async fn record_visit(db: &mut Connection<SinSuanDB>, domain: String, visit_record: VisitRecord) -> Result<(), sqlx::Error> {
  let domain_table_name = get_table_name("visit_record", domain.as_str());
  // 插入访问记录
  let result = sqlx::query(
    &format!(
        "INSERT INTO {} (path, user_id, timestamp) VALUES ($1, $2, CURRENT_TIMESTAMP)",
        domain_table_name
      )
    )
    .bind(visit_record.path)
    .bind(visit_record.user_id)
    .execute(db.as_mut())
    .await.unwrap();

  println!("insert result: {:?}", result.rows_affected());

  Ok(())
}