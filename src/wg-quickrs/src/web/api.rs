use crate::conf;
use crate::wireguard;
use crate::mode::ui_mode;
use crate::web::init;
use actix_web::{HttpRequest, HttpResponse, Responder, get, patch, post, web};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use once_cell::sync::Lazy;
use rand::{RngCore, rng};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use wg_quickrs_lib::types::misc::VERSION_BUILD_INFO;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Claims {
    sub: String, // Subject (user id)
    exp: u64,    // Expiration time as a timestamp
}

// Secret key for signing tokens
static JWT_SECRETS: Lazy<(EncodingKey, DecodingKey)> = Lazy::new(|| {
    let mut key = [0u8; 32];
    rng().fill_bytes(&mut key);
    (
        EncodingKey::from_secret(&key),
        DecodingKey::from_secret(&key),
    )
});

#[get("/api/version")]
async fn get_version(req: HttpRequest) -> impl Responder {
    if let Err(e) = enforce_auth(req) {
        return e;
    }

    HttpResponse::Ok().json(VERSION_BUILD_INFO)
}

#[derive(serde::Deserialize)]
pub(crate) struct SummaryBody {
    #[serde(default)]
    pub(crate) only_digest: bool,
}

#[get("/api/network/summary")]
async fn get_network_summary(req: HttpRequest, query: web::Query<SummaryBody>) -> impl Responder {
    if let Err(e) = enforce_auth(req) {
        return e;
    }
    conf::respond::get_network_summary(query).unwrap_or_else(|e| e)
}

#[patch("/api/network/config")]
async fn patch_network_config(req: HttpRequest, body: web::Bytes) -> impl Responder {
    if let Err(e) = enforce_auth(req) {
        return e;
    }
    conf::respond::patch_network_config(body).unwrap_or_else(|e| e)
}

#[post("/api/network/reserve/address")]
async fn post_network_reserve_address(req: HttpRequest) -> impl Responder {
    if let Err(e) = enforce_auth(req) {
        return e;
    }
    conf::respond::post_network_reserve_address().unwrap_or_else(|e| e)
}

#[post("/api/wireguard/status")]
async fn post_wireguard_status(req: HttpRequest, body: web::Bytes) -> impl Responder {
    if let Err(e) = enforce_auth(req) {
        return e;
    }
    wireguard::respond::post_wireguard_server_status(body).unwrap_or_else(|e| e)
}

// Mode endpoints
#[get("/api/mode")]
async fn get_mode(req: HttpRequest) -> impl Responder {
    if let Err(e) = enforce_auth(req.clone()) {
        return e;
    }
    ui_mode::get_mode(req).await
}

#[patch("/api/mode/toggle")]
async fn patch_mode_toggle(req: HttpRequest, body: web::Bytes) -> impl Responder {
    if let Err(e) = enforce_auth(req.clone()) {
        return e;
    }
    ui_mode::toggle_mode(req, body).await
}

#[get("/api/mode/can-switch")]
async fn get_mode_can_switch(req: HttpRequest) -> impl Responder {
    if let Err(e) = enforce_auth(req.clone()) {
        return e;
    }
    ui_mode::can_switch_mode(req).await
}

#[patch("/api/mode/peer-route-status")]
async fn patch_peer_route_status(req: HttpRequest, body: web::Bytes) -> impl Responder {
    if let Err(e) = enforce_auth(req.clone()) {
        return e;
    }
    ui_mode::update_peer_route_status(req, body).await
}

#[get("/api/mode/exit-node")]
async fn get_exit_node_info(req: HttpRequest) -> impl Responder {
    if let Err(e) = enforce_auth(req.clone()) {
        return e;
    }
    ui_mode::get_exit_node_info(req).await
}

#[post("/api/peer/control")]
async fn post_peer_control(req: HttpRequest, body: web::Bytes) -> impl Responder {
    if let Err(e) = enforce_auth(req.clone()) {
        return e;
    }
    ui_mode::peer_control(req, body).await
}

#[patch("/api/peer/lan-access")]
async fn patch_peer_lan_access(req: HttpRequest, body: web::Bytes) -> impl Responder {
    if let Err(e) = enforce_auth(req.clone()) {
        return e;
    }
    ui_mode::set_peer_lan_access(req, body).await
}

#[get("/api/peer/lan-access")]
async fn get_peer_lan_access(req: HttpRequest) -> impl Responder {
    if let Err(e) = enforce_auth(req.clone()) {
        return e;
    }
    ui_mode::get_peer_lan_access_all(req).await
}

#[get("/api/router-mode/auto-failover")]
pub async fn get_auto_failover(req: HttpRequest) -> impl Responder {
    if let Err(e) = enforce_auth(req.clone()) {
        return e;
    }
    ui_mode::get_auto_failover(req).await
}

#[post("/api/router-mode/auto-failover")]
pub async fn post_auto_failover(req: HttpRequest, body: web::Bytes) -> impl Responder {
    if let Err(e) = enforce_auth(req.clone()) {
        return e;
    }
    ui_mode::set_auto_failover(req, body).await
}

#[derive(serde::Deserialize)]
pub(crate) struct LogsQuery {
    #[serde(default = "default_log_lines")]
    pub(crate) lines: usize,
}

fn default_log_lines() -> usize {
    100
}

#[get("/api/system/logs")]
pub async fn get_system_logs(req: HttpRequest, query: web::Query<LogsQuery>) -> impl Responder {
    if let Err(e) = enforce_auth(req) {
        return e;
    }
    
    // Fetch logs from journalctl for wg-quickrs service
    let lines = query.lines.min(1000); // Cap at 1000 lines
    
    match std::process::Command::new("journalctl")
        .args(["-u", "wg-quickrs", "-n", &lines.to_string(), "--no-pager", "-o", "short-iso"])
        .output()
    {
        Ok(output) => {
            let logs = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            
            if !output.status.success() && logs.is_empty() {
                // Try alternative: read from /var/log if journalctl fails
                HttpResponse::Ok().json(serde_json::json!({
                    "logs": format!("journalctl error: {}", stderr),
                    "source": "journalctl",
                    "lines": 0
                }))
            } else {
                let line_count = logs.lines().count();
                HttpResponse::Ok().json(serde_json::json!({
                    "logs": logs,
                    "source": "journalctl",
                    "lines": line_count
                }))
            }
        }
        Err(e) => {
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to fetch logs: {}", e)
            }))
        }
    }
}

// Init endpoints (no auth required - used before config exists)
#[get("/api/init/status")]
async fn get_init_status(_req: HttpRequest) -> impl Responder {
    init::get_init_status(_req).await
}

#[get("/api/init/info")]
async fn get_init_info(_req: HttpRequest) -> impl Responder {
    init::get_init_info(_req).await
}

#[post("/api/init")]
async fn post_init(_req: HttpRequest, body: web::Bytes) -> impl Responder {
    init::post_init(_req, body).await
}

#[post("/api/token")]
async fn post_token(body: web::Bytes) -> impl Responder {
    // check password-based auth
    let config = match conf::util::get_config() {
        Ok(config) => config,
        Err(_) => {
            return HttpResponse::InternalServerError().body("Unable to get config");
        }
    };
    if !config.agent.web.password.enabled {
        return HttpResponse::NoContent().body("Token authentication not enabled");
    }

    #[derive(Serialize, Deserialize)]
    struct LoginBody {
        client_id: String,
        password: String,
    }
    let body_raw = String::from_utf8_lossy(&body);
    let status_body: LoginBody = match serde_json::from_str(&body_raw) {
        Ok(val) => val,
        Err(err) => {
            return HttpResponse::BadRequest().body(format!("invalid JSON: {err}"));
        }
    };
    let client_id = &status_body.client_id;
    let password = &status_body.password;

    // check password-based auth
    let parsed_hash = match PasswordHash::new(&config.agent.web.password.hash) {
        Ok(hash) => hash,
        Err(e) => {
            log::error!("Invalid password hash format in configuration: {}", e);
            return HttpResponse::InternalServerError().body("Server configuration error");
        }
    };
    if Argon2::default().verify_password(password.as_bytes(), &parsed_hash).is_err() {
        return HttpResponse::Unauthorized().body("Invalid credentials");
    }

    let expiration = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_secs() + 3600, // 1-hour expiry
        Err(_) => return HttpResponse::InternalServerError().body("SystemTime before UNIX EPOCH!"),
    };

    let claims = Claims {
        sub: client_id.clone(),
        exp: expiration,
    };

    match encode(&Header::default(), &claims, &JWT_SECRETS.0) {
        Ok(token) => HttpResponse::Ok().body(token),
        Err(_) => HttpResponse::InternalServerError().body("Token creation error"),
    }
}

fn enforce_auth(req: HttpRequest) -> Result<(), HttpResponse> {
    // check password-based auth
    let config = match conf::util::get_config() {
        Ok(config) => config,
        Err(_) => {
            return Err(HttpResponse::InternalServerError().body("Unable to get config"));
        }
    };
    if !config.agent.web.password.enabled {
        return Ok(());
    }

    if let Some(auth_header) = req.headers().get("Authorization")
        && let Ok(auth_str) = auth_header.to_str()
        && let Some(token) = auth_str.strip_prefix("Bearer ")
    {
        let validation = Validation::new(Algorithm::HS256);

        return match decode::<Claims>(token, &JWT_SECRETS.1, &validation) {
            Ok(_) => Ok(()),
            Err(_) => Err(HttpResponse::Unauthorized()
                .content_type("text/plain; charset=utf-8")
                .body("Invalid token")),
        };
    }

    Err(HttpResponse::Unauthorized()
        .content_type("text/plain; charset=utf-8")
        .body("Authorization header missing or invalid"))
}
