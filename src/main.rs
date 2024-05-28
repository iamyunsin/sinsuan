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

use rocket::fairing::AdHoc;
use rocket_db_pools::Database;
mod lib;
mod routes;
mod app_config;

use lib::cors::CORS;
use lib::storage::SinSuanDB;
use routes as my_routes;



#[launch]
async fn rocket() -> _ {
  rocket::build()
    .attach(CORS)
    .attach(SinSuanDB::init())
    .mount(
      "/",
      routes![
        my_routes::count::get_count,
        my_routes::count::count_cors
        ]
      )
      .attach(AdHoc::config::<app_config::Config>())
}
