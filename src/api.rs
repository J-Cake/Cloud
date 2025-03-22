use crate::{
	Args,
	HTTPClient
};
use actix_session::{
	SessionMiddleware
};
use actix_web::{
    Responder,
    HttpResponseBuilder,
    HttpResponse,
    Error,
    Result,
    body::MessageBody,
    cookie::Cookie,
    dev::ServiceRequest,
    dev::ServiceResponse,
    error::JsonPayloadError,
    error::PayloadError,
    get,
    middleware::Next,
    post,
    web,
    Route,
    Scope,
    HttpMessage,
    HttpRequest,
    body::BoxBody,
    dev::{Service, ServiceFactory, Transform},
    middleware::from_fn,
    web::Data
};
use base64::{
	Engine,
	prelude::BASE64_STANDARD
};
use log::error;
use rand::RngCore;
use reqwest::{
	Client,
	Url
};
use serde::{
	Deserialize,
	Serialize
};
use serde_json::{
    json,
    Value
};
use sqlx::{
    FromRow,
    PgPool,
    Row,
    postgres::PgRow
};
use std::{
	cell::Cell,
	cell::LazyCell,
	cell::RefCell
};
use actix_session::config::SessionMiddlewareBuilder;

thread_local! {
    pub static RNG: RefCell<rand::rngs::ThreadRng> = RefCell::new(rand::rng());
}

#[derive(Clone, Serialize, Deserialize)]
pub struct OAuthConfig {
    pub token_url: String,
    pub user_url: String,
    client_id: String,
    client_secret: String,
}

impl OAuthConfig {
    pub fn token(&self, token: impl AsRef<str>) -> OAuthBody {
        OAuthBody {
            client_id: self.client_id.clone(),
            client_secret: self.client_secret.clone(),
            code: token.as_ref().to_owned(),
            accept: "application/json".to_string(),
        }
    }
}

#[derive(Deserialize)]
struct OAuthRequest {
    code: String,
}

#[derive(Serialize)]
struct OAuthBody {
    client_id: String,
    client_secret: String,
    code: String,

    accept: String,
}

#[derive(Deserialize, Debug)]
struct OAuthResponse {
    access_token: String,
    scope: String,
    token_type: String,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct User {
    email: String,
    #[serde(rename = "displayName")]
    display: String,
}

#[get("/login")]
pub async fn login(pool: web::Data<PgPool>, client: web::Data<HTTPClient>, oauth: web::Data<OAuthConfig>, query: web::Query<OAuthRequest>) -> Result<impl Responder> {
    let result = match client
        .post_json::<OAuthResponse, _>(&oauth.token_url, &oauth.token(&query.code), None::<String>)
        .await
    {
        Ok(resp) => resp,
        Err(err) => {
            error!("{:?}", &err);
            return Ok(HttpResponse::UnprocessableEntity().json(json! {{
                "success": false,
                "msg": "Could not receive code",
                "err": err.to_string()
            }}));
        }
    };

    let user = match client.get_json::<Value>(&oauth.user_url, Some(&result.access_token)).await {
        Ok(user) => User {
            email: match user.get("email").and_then(Value::as_str) {
                Some(email) => email.to_owned(),
                None =>
                    return Ok(HttpResponse::ExpectationFailed().json(serde_json::json! {{
                        "success": false,
                        "msg": "Expected `email`"
                    }})),
            },
            display: match user.get("name").and_then(Value::as_str) {
                Some(name) => name.to_owned(),
                None =>
                    return Ok(HttpResponse::ExpectationFailed().json(serde_json::json! {{
                        "success": false,
                        "msg": "Expected `name`"
                    }})),
            },
        },
        Err(err) => {
            error!("{:?}", err);
            return Ok(HttpResponse::UnprocessableEntity().json(serde_json::json! {{
                "success": false,
                "msg": "Failed to receive response from OAuth provider",
                "err": err.to_string()
            }}));
        }
    };

    let token = RNG.with_borrow_mut(|rng| BASE64_STANDARD.encode((0u64..16).map(|_| rng.next_u64().to_ne_bytes()).flatten().collect::<Vec<u8>>()));

    if let Err(err) = sqlx::query(
        r#"WITH new
         AS (INSERT INTO users ("email", "display") VALUES ($1, $2) ON CONFLICT (email) DO UPDATE SET display = users.display RETURNING *)
INSERT
INTO oauth_keys ("user", "token")
SELECT uid as "user", $3 as token
FROM new"#,
    )
    .bind(&user.email)
    .bind(&user.display)
    .bind(&token)
    .execute(pool.get_ref())
    .await
    {
        error!("{:?}", &err);
        return Ok(HttpResponse::InternalServerError().json(json! {{
            "success": false,
            "msg": "Failed to acquire user token."
        }}));
    };

    Ok(HttpResponse::Ok().json(json! {{
        "success": true,
        "token": token,
        "expiry": null,
        "user": user
    }}))
}

pub async fn authenticate(req: ServiceRequest, next: Next<impl MessageBody + 'static>) -> Result<ServiceResponse<impl MessageBody>> {
    let Some(token) = req.headers().get("Authorization") else {
        return Ok(req.into_response(HttpResponse::Unauthorized().json(json! {{
            "success": false,
            "msg": "Authorisation header not present."
        }})));
    };

    let token = match token.to_str() {
        Ok(token) if token.to_lowercase().starts_with("bearer ") => token[7..].to_owned(),
        _ => return Ok(req.into_response(HttpResponse::BadRequest()
            .json(json! {{
                "success": false,
                "msg": "Invalid token"
            }})))
    };

    let Some(pool) = req.app_data::<Data<PgPool>>() else {
        return Ok(req.into_response(HttpResponse::InternalServerError().json(json! {{
            "success": false,
            "msg": "Could not acquire database connection."
        }})));
    };

    let user: User = match sqlx::query(r#"SELECT * FROM oauth_keys LEFT OUTER JOIN users ON users.uid = oauth_keys."user" WHERE token = $1"#)
        .bind(token)
        .fetch_one(pool.get_ref())
        .await {
        Ok(user) => match User::from_row(&user) {
            Ok(user) => user,
            Err(err) => {
                log::error!("{:?}", err);
                return Ok(req.into_response(HttpResponse::InternalServerError().json(json! {{
                    "success": false,
                    "msg": "Corrupt database user."
                }})))
            }
        },
        Err(err) => {
            error!("{:?}", err);
            return Ok(req.into_response(HttpResponse::Unauthorized().json(json! {{
                "success": false,
                "msg": "No valid token was found."
            }})));
        }
    };

    req.extensions_mut().insert(user);

    Ok(next.call(req).await?.map_into_boxed_body())
}

#[get("/user")]
pub async fn get_user(req: HttpRequest) ->  Result<impl Responder> {
    let ext = req.extensions();
    let Some(user) = ext.get::<User>() else {
        return Ok(HttpResponse::NotFound().json(json! {{
            "success": true,
            "msg": "User not found."
        }}))
    };

    Ok(HttpResponse::Ok().json(json! {{
        "success": true,
        "user": user
    }}))
}