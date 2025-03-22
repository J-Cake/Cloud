mod sql;
mod api;
mod app;

use crate::{
    api::OAuthConfig,
    sql::SqlMap
};
use actix_files::{
    Files,
    NamedFile
};
use actix_web::{
    middleware,
    web,
    web::Data,
    App,
    HttpServer,
    Responder
};
use reqwest::{
    Method,
    RequestBuilder
};
use serde::{
    Deserialize,
    Serialize
};
use std::{
    fs,
    net::SocketAddr,
    ops::Deref,
    path::PathBuf
};
use actix_web::middleware::from_fn;

#[derive(Clone)]
pub struct HTTPClient {
    client: reqwest::Client,
}

impl Deref for HTTPClient {
    type Target = reqwest::Client;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl HTTPClient {
    fn req(&self, url: impl AsRef<str>, method: reqwest::Method, token: Option<impl AsRef<str>>) -> RequestBuilder {
        let mut req = self.client.request(method.into(), url.as_ref())
            .header("Content-Type", "application/json")
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "jcake-cloud");

        if let Some(token) = token {
            req = req.bearer_auth(token.as_ref())
        }

        return req;
    }

    pub async fn get_json<Response: for<'a> Deserialize<'a>>(&self, url: impl AsRef<str>, token: Option<impl AsRef<str>>) -> reqwest::Result<Response> {
        Ok(self.req(url, Method::GET, token)
            .send()
            .await?
            .json::<Response>()
            .await?)
    }

    pub async fn get_text(&self, url: impl AsRef<str>, token: Option<impl AsRef<str>>) -> reqwest::Result<String> {
        Ok(self.req(url, Method::GET, token)
            .send()
            .await?
            .text()
            .await?)
    }

    pub async fn post_json<Response: for<'a> Deserialize<'a>, Body: Serialize>(&self, url: impl AsRef<str>, body: Body, token: Option<impl AsRef<str>>) -> reqwest::Result<Response> {
        Ok(self.req(url, Method::POST, token)
            .json(&body)
            .send()
            .await?
            .json::<Response>()
            .await?)
    }

    pub async fn post_text<Body: Serialize>(&self, url: impl AsRef<str>, body: Body, token: Option<impl AsRef<str>>) -> reqwest::Result<String> {
        Ok(self.req(url, Method::POST, token)
            .json(&body)
            .send()
            .await?
            .text()
            .await?)
    }
}

#[derive(clap::Parser)]
#[derive(Clone)]
struct Args  {
    #[clap(short, long)]
    listen: SocketAddr,

    #[clap(short, long)]
    database: String,

    #[clap(short, long)]
    sql: PathBuf,

    #[clap(long, default_value = "./oauth.json")]
    oauth_config: PathBuf,

    #[clap(long, default_value = "./static")]
    r#static: PathBuf,

    #[clap(long, default_value = "index.html")]
    index: PathBuf
}

#[actix_web::main]
async fn main() -> actix_web::Result<()> {
    env_logger::init();
    let args: Args = clap::Parser::parse();

    if !args.index.exists() || !args.r#static.exists() {
        panic!("Invalid Resource configuration. Please check `./index.html` and `./static`");
    }

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(32)
        .connect(args.database.as_str())
        .await
        .expect("Could not connect to database");

    let addr = args.listen.clone();
    let sql_map = SqlMap::new(args.sql.clone())?;

    let client = HTTPClient { client: reqwest::Client::new() };

    let oauth_config: OAuthConfig = serde_json::from_reader(fs::OpenOptions::new()
        .read(true)
        .open(&args.oauth_config)?)?;

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(sql_map.clone()))
            .app_data(web::Data::new(args.clone()))
            .app_data(web::Data::new(client.clone()))
            .app_data(web::Data::new(oauth_config.clone()))
            .wrap(middleware::DefaultHeaders::new().add(("X-Version", "0.2")))
            .service(sql::method)
            .service(api::login)
            .service(web::scope("/api")
                .wrap(from_fn(api::authenticate))
                .service(api::get_user))
            .service(Files::new("/static", &args.r#static).prefer_utf8(true))
            .route("/app", web::to(async |args: Data<Args>| NamedFile::open(&args.index)))
            .route("/app/{suburl:.*}", web::to(async |args: Data<Args>| NamedFile::open(&args.index)))
    })
        .workers(std::thread::available_parallelism().expect("Failed to get CPUs").get())
        .bind(addr)?
        .run()
        .await?;

    Ok(())
}