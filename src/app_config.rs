use rocket::serde::{Serialize, Deserialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(crate = "rocket::serde")]
pub struct Config {
  pub qq_map: QQMapConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct QQMapConfig {
  pub base_url: String,
  pub key: String,
  pub sk: String,
}