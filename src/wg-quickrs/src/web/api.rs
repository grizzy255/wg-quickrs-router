use crate::conf;
use crate::wireguard;
use crate::mode::ui_mode;
use crate::web::init;
use actix_web::{HttpRequest, HttpResponse, Responder, get, patch, post, web};
use argon2::{Argon2, PasswordHash, PasswordVerifier, PasswordHasher, password_hash::SaltString};
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

#[get("/api/system/routing")]
pub async fn get_routing_info(req: HttpRequest) -> impl Responder {
    if let Err(e) = enforce_auth(req) {
        return e;
    }
    
    use std::collections::HashMap;
    
    // Get IP rules
    let ip_rules = match std::process::Command::new("ip")
        .args(["rule", "show"])
        .output()
    {
        Ok(output) => String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|s| s.to_string())
            .collect::<Vec<_>>(),
        Err(_) => vec![]
    };
    
    // Get main routing table
    let routes_main = match std::process::Command::new("ip")
        .args(["route", "show", "table", "main"])
        .output()
    {
        Ok(output) => String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|s| s.to_string())
            .collect::<Vec<_>>(),
        Err(_) => vec![]
    };
    
    // Find WireGuard-related routing tables from ip rules
    let mut routes_wg: HashMap<String, Vec<String>> = HashMap::new();
    for rule in &ip_rules {
        // Look for table numbers or names that might be WireGuard-related
        if let Some(table) = rule.split_whitespace()
            .skip_while(|&w| w != "lookup")
            .nth(1)
        {
            // Skip main, local, default tables
            if table == "main" || table == "local" || table == "default" {
                continue;
            }
            // Get routes for this table if not already fetched
            if !routes_wg.contains_key(table) {
                if let Ok(output) = std::process::Command::new("ip")
                    .args(["route", "show", "table", table])
                    .output()
                {
                    let routes: Vec<String> = String::from_utf8_lossy(&output.stdout)
                        .lines()
                        .map(|s| s.to_string())
                        .collect();
                    if !routes.is_empty() {
                        routes_wg.insert(table.to_string(), routes);
                    }
                }
            }
        }
    }
    
    // Get WireGuard peer allowed IPs
    let mut peer_allowed_ips: HashMap<String, String> = HashMap::new();
    let mut interface_name = String::from("WireStream");
    
    // First try to get the WireGuard interface name
    if let Ok(output) = std::process::Command::new("wg")
        .args(["show", "interfaces"])
        .output()
    {
        let interfaces = String::from_utf8_lossy(&output.stdout);
        if let Some(iface) = interfaces.split_whitespace().next() {
            interface_name = iface.to_string();
        }
    }
    
    // Get allowed-ips for the interface
    if let Ok(output) = std::process::Command::new("wg")
        .args(["show", &interface_name, "allowed-ips"])
        .output()
    {
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let public_key = parts[0].to_string();
                let allowed = parts[1..].join(", ");
                peer_allowed_ips.insert(public_key, allowed);
            } else if parts.len() == 1 {
                peer_allowed_ips.insert(parts[0].to_string(), "(none)".to_string());
            }
        }
    }
    
    // Get default gateway
    let default_gateway = routes_main.iter()
        .find(|r| r.starts_with("default"))
        .cloned();
    
    HttpResponse::Ok().json(serde_json::json!({
        "ip_rules": ip_rules,
        "routes_main": routes_main,
        "routes_wg": routes_wg,
        "peer_allowed_ips": peer_allowed_ips,
        "interface": interface_name,
        "default_gateway": default_gateway
    }))
}

// Diagnostics endpoints

// Helper to get the WireGuard interface name
fn get_wg_interface() -> Option<String> {
    std::process::Command::new("wg")
        .args(["show", "interfaces"])
        .output()
        .ok()
        .and_then(|out| {
            String::from_utf8_lossy(&out.stdout)
                .split_whitespace()
                .next()
                .map(|s| s.to_string())
        })
}

// Helper to get current exit node info for display
fn get_current_exit_node_info() -> String {
    // Try to get the current exit node from the mode state
    if let Ok(config) = crate::conf::util::get_config() {
        // Get exit node info from the routing state - look for default route (0.0.0.0/0)
        if let Ok(Some(state)) = crate::mode::persist::load_mode_state() {
            if let Some(prefix_state) = state.prefix_active_backup.get("0.0.0.0/0") {
                let active_peer_id = &prefix_state.active_peer_id;
                // Try to parse as UUID and look up peer name
                if let Ok(peer_uuid) = uuid::Uuid::parse_str(active_peer_id) {
                    if let Some(peer) = config.network.peers.get(&peer_uuid) {
                        return peer.name.clone();
                    }
                }
                // Fallback: truncate the ID
                return active_peer_id.chars().take(8).collect::<String>() + "...";
            }
        }
    }
    "unknown gateway".to_string()
}

// Helper to validate target string
fn validate_target(target: &str) -> bool {
    !target.is_empty() 
        && target.len() <= 253 
        && !target.contains(';') 
        && !target.contains('&') 
        && !target.contains('|')
        && !target.contains('`')
        && !target.contains('$')
}

#[derive(serde::Deserialize)]
pub struct DiagnosticsPingRequest {
    pub target: String,
    #[serde(default = "default_ping_count")]
    pub count: u32,
    #[serde(default)]
    pub interface: Option<String>, // "wg" for WireGuard, empty/null for default route
}

fn default_ping_count() -> u32 { 4 }

#[post("/api/system/diagnostics/ping")]
pub async fn post_diagnostics_ping(req: HttpRequest, body: web::Json<DiagnosticsPingRequest>) -> impl Responder {
    if let Err(e) = enforce_auth(req) {
        return e;
    }
    
    let target = &body.target;
    let count = body.count.min(20); // Cap at 20 pings
    
    if !validate_target(target) {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Invalid target"
        }));
    }
    
    let mut args = vec!["-c".to_string(), count.to_string(), "-W".to_string(), "5".to_string()];
    
    // Add interface if specified
    let mut via_info = "default route (LAN)".to_string();
    if let Some(ref iface) = body.interface {
        if iface == "wg" {
            if let Some(wg_iface) = get_wg_interface() {
                args.push("-I".to_string());
                args.push(wg_iface.clone());
                let exit_node = get_current_exit_node_info();
                via_info = format!("WireGuard → {}", exit_node);
            }
        } else if !iface.is_empty() {
            args.push("-I".to_string());
            args.push(iface.clone());
            via_info = format!("interface {}", iface);
        }
    }
    
    args.push(target.clone());
    
    match std::process::Command::new("ping")
        .args(&args)
        .output()
    {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            HttpResponse::Ok().json(serde_json::json!({
                "success": output.status.success(),
                "output": stdout,
                "error": if stderr.is_empty() { None } else { Some(stderr) },
                "via": via_info
            }))
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to execute ping: {}", e)
        }))
    }
}

#[derive(serde::Deserialize)]
pub struct DiagnosticsTracerouteRequest {
    pub target: String,
    #[serde(default = "default_max_hops")]
    pub max_hops: u32,
    #[serde(default)]
    pub interface: Option<String>,
}

fn default_max_hops() -> u32 { 20 }

#[post("/api/system/diagnostics/traceroute")]
pub async fn post_diagnostics_traceroute(req: HttpRequest, body: web::Json<DiagnosticsTracerouteRequest>) -> impl Responder {
    if let Err(e) = enforce_auth(req) {
        return e;
    }
    
    let target = &body.target;
    let max_hops = body.max_hops.min(30);
    
    if !validate_target(target) {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Invalid target"
        }));
    }
    
    let mut via_info = "default route (LAN)".to_string();
    let interface_opt = if let Some(ref iface) = body.interface {
        if iface == "wg" {
            if let Some(wg_iface) = get_wg_interface() {
                let exit_node = get_current_exit_node_info();
                via_info = format!("WireGuard → {}", exit_node);
                Some(wg_iface)
            } else {
                None
            }
        } else if !iface.is_empty() {
            via_info = format!("interface {}", iface);
            Some(iface.clone())
        } else {
            None
        }
    } else {
        None
    };
    
    // Try traceroute first, fall back to tracepath
    let mut args = vec!["-m".to_string(), max_hops.to_string(), "-w".to_string(), "3".to_string()];
    if let Some(ref iface) = interface_opt {
        args.push("-i".to_string());
        args.push(iface.clone());
    }
    args.push(target.clone());
    
    let result = std::process::Command::new("traceroute")
        .args(&args)
        .output();
    
    let (output, tool) = match result {
        Ok(out) if out.status.success() || !String::from_utf8_lossy(&out.stdout).is_empty() => (out, "traceroute"),
        _ => {
            // Try tracepath as fallback (doesn't support -i)
            let mut tp_args = vec!["-m".to_string(), max_hops.to_string()];
            tp_args.push(target.clone());
            match std::process::Command::new("tracepath")
                .args(&tp_args)
                .output()
            {
                Ok(out) => (out, "tracepath"),
                Err(e) => {
                    return HttpResponse::InternalServerError().json(serde_json::json!({
                        "error": format!("Neither traceroute nor tracepath available: {}", e)
                    }));
                }
            }
        }
    };
    
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    HttpResponse::Ok().json(serde_json::json!({
        "success": output.status.success(),
        "output": stdout,
        "error": if stderr.is_empty() { None } else { Some(stderr) },
        "tool": tool,
        "via": via_info
    }))
}

#[derive(serde::Deserialize)]
pub struct DiagnosticsDnsRequest {
    pub hostname: String,
    #[serde(default)]
    pub server: Option<String>,
    #[serde(default)]
    pub record_type: Option<String>,
}

#[post("/api/system/diagnostics/dns")]
pub async fn post_diagnostics_dns(req: HttpRequest, body: web::Json<DiagnosticsDnsRequest>) -> impl Responder {
    if let Err(e) = enforce_auth(req) {
        return e;
    }
    
    let hostname = &body.hostname;
    
    if !validate_target(hostname) {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Invalid hostname"
        }));
    }
    
    let server_info = body.server.clone()
        .filter(|s| !s.is_empty())
        .map(|s| format!("DNS server: {}", s))
        .unwrap_or_else(|| "system DNS".to_string());
    
    let mut args = vec![hostname.clone()];
    
    // Add record type if specified
    if let Some(ref rt) = body.record_type {
        if !rt.is_empty() && validate_target(rt) {
            args.push(rt.clone());
        }
    }
    
    // Add DNS server if specified
    if let Some(ref server) = body.server {
        if !server.is_empty() && validate_target(server) {
            args.push(format!("@{}", server));
        }
    }
    
    args.push("+short".to_string());
    args.push("+timeout=5".to_string());
    
    // Try dig first, then nslookup, then host
    let result = std::process::Command::new("dig")
        .args(&args)
        .output();
    
    let (output, tool) = match result {
        Ok(out) if out.status.success() || !String::from_utf8_lossy(&out.stdout).is_empty() => (out, "dig"),
        _ => {
            // Try nslookup
            let mut nslookup_args = vec![hostname.clone()];
            if let Some(ref server) = body.server {
                if !server.is_empty() {
                    nslookup_args.push(server.clone());
                }
            }
            match std::process::Command::new("nslookup")
                .args(&nslookup_args)
                .output()
            {
                Ok(out) => (out, "nslookup"),
                Err(_) => {
                    // Try host command
                    match std::process::Command::new("host")
                        .arg(hostname)
                        .output()
                    {
                        Ok(out) => (out, "host"),
                        Err(e) => {
                            return HttpResponse::InternalServerError().json(serde_json::json!({
                                "error": format!("No DNS tools available: {}", e)
                            }));
                        }
                    }
                }
            }
        }
    };
    
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    HttpResponse::Ok().json(serde_json::json!({
        "success": output.status.success(),
        "output": stdout,
        "error": if stderr.is_empty() { None } else { Some(stderr) },
        "tool": tool,
        "via": server_info
    }))
}

#[derive(serde::Deserialize)]
pub struct DiagnosticsMtuRequest {
    pub target: String,
    #[serde(default = "default_mtu_start")]
    pub start_size: u32,
    #[serde(default = "default_mtu_end")]
    pub end_size: u32,
    #[serde(default)]
    pub interface: Option<String>,
}

fn default_mtu_start() -> u32 { 1500 }
fn default_mtu_end() -> u32 { 1280 }

#[post("/api/system/diagnostics/mtu")]
pub async fn post_diagnostics_mtu(req: HttpRequest, body: web::Json<DiagnosticsMtuRequest>) -> impl Responder {
    if let Err(e) = enforce_auth(req) {
        return e;
    }
    
    let target = &body.target;
    let start = body.start_size.min(1500);
    let end = body.end_size.max(500);
    
    if !validate_target(target) {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Invalid target"
        }));
    }
    
    // Determine interface
    let mut via_info = "default route (LAN)".to_string();
    let interface_args: Vec<String> = if let Some(ref iface) = body.interface {
        if iface == "wg" {
            if let Some(wg_iface) = get_wg_interface() {
                let exit_node = get_current_exit_node_info();
                via_info = format!("WireGuard → {}", exit_node);
                vec!["-I".to_string(), wg_iface]
            } else {
                vec![]
            }
        } else if !iface.is_empty() {
            via_info = format!("interface {}", iface);
            vec!["-I".to_string(), iface.clone()]
        } else {
            vec![]
        }
    } else {
        vec![]
    };
    
    let mut results = Vec::new();
    let mut optimal_mtu: u32 = 0;
    
    // Binary search for optimal MTU
    let mut low = end;
    let mut high = start;
    
    while low <= high {
        let mid = (low + high) / 2;
        // Packet size = MTU - 28 (20 bytes IP header + 8 bytes ICMP header)
        let packet_size = mid.saturating_sub(28);
        
        let mut args = vec!["-c".to_string(), "1".to_string(), "-W".to_string(), "3".to_string(), 
                          "-M".to_string(), "do".to_string(), "-s".to_string(), packet_size.to_string()];
        args.extend(interface_args.clone());
        args.push(target.clone());
        
        let output = std::process::Command::new("ping")
            .args(&args)
            .output();
        
        let success = match output {
            Ok(out) => out.status.success(),
            Err(_) => false,
        };
        
        results.push(serde_json::json!({
            "mtu": mid,
            "packet_size": packet_size,
            "success": success
        }));
        
        if success {
            optimal_mtu = mid;
            low = mid + 1;
        } else {
            high = mid - 1;
        }
    }
    
    HttpResponse::Ok().json(serde_json::json!({
        "optimal_mtu": optimal_mtu,
        "results": results,
        "via": via_info,
        "recommendation": if optimal_mtu > 0 {
            format!("Recommended MTU: {} (allows {} byte packets)", optimal_mtu, optimal_mtu - 28)
        } else {
            "Could not determine optimal MTU. Target may be unreachable.".to_string()
        }
    }))
}

#[derive(serde::Deserialize)]
pub struct DiagnosticsPeerCheckRequest {
    pub peer_id: String,
}

#[post("/api/system/diagnostics/peer-check")]
pub async fn post_diagnostics_peer_check(req: HttpRequest, body: web::Json<DiagnosticsPeerCheckRequest>) -> impl Responder {
    if let Err(e) = enforce_auth(req) {
        return e;
    }
    
    let peer_id = &body.peer_id;
    let mut checks = Vec::new();
    
    // 1. Check if WireGuard interface is up
    let wg_status = std::process::Command::new("wg")
        .args(["show", "WireStream"])
        .output();
    
    let wg_up = match wg_status {
        Ok(out) => out.status.success(),
        Err(_) => false,
    };
    checks.push(serde_json::json!({
        "name": "WireGuard Interface",
        "status": if wg_up { "ok" } else { "error" },
        "message": if wg_up { "WireStream interface is active" } else { "WireStream interface is down or not found" }
    }));
    
    // 2. Check peer in WireGuard config
    let peer_in_wg = if wg_up {
        let output = std::process::Command::new("wg")
            .args(["show", "WireStream", "peers"])
            .output();
        match output {
            Ok(out) => {
                let peers = String::from_utf8_lossy(&out.stdout);
                !peers.is_empty()
            }
            Err(_) => false,
        }
    } else {
        false
    };
    checks.push(serde_json::json!({
        "name": "Peer Configuration",
        "status": if peer_in_wg { "ok" } else { "warning" },
        "message": if peer_in_wg { "Peers are configured in WireGuard" } else { "No peers configured" }
    }));
    
    // 3. Check for recent handshake
    let handshake_ok = if wg_up {
        let output = std::process::Command::new("wg")
            .args(["show", "WireStream", "latest-handshakes"])
            .output();
        match output {
            Ok(out) => {
                let handshakes = String::from_utf8_lossy(&out.stdout);
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                
                // Check if any handshake is within last 3 minutes
                handshakes.lines().any(|line| {
                    if let Some(ts_str) = line.split_whitespace().nth(1) {
                        if let Ok(ts) = ts_str.parse::<u64>() {
                            return ts > 0 && now - ts < 180;
                        }
                    }
                    false
                })
            }
            Err(_) => false,
        }
    } else {
        false
    };
    checks.push(serde_json::json!({
        "name": "Recent Handshake",
        "status": if handshake_ok { "ok" } else { "warning" },
        "message": if handshake_ok { "Recent handshake detected (within 3 minutes)" } else { "No recent handshake - peer may be offline or unreachable" }
    }));
    
    // 4. Check IP forwarding (for router mode)
    let ip_forward = std::fs::read_to_string("/proc/sys/net/ipv4/ip_forward")
        .map(|s| s.trim() == "1")
        .unwrap_or(false);
    checks.push(serde_json::json!({
        "name": "IP Forwarding",
        "status": if ip_forward { "ok" } else { "warning" },
        "message": if ip_forward { "IP forwarding is enabled" } else { "IP forwarding is disabled (required for router mode)" }
    }));
    
    // 5. Check NAT/masquerade rules
    let nat_ok = std::process::Command::new("iptables")
        .args(["-t", "nat", "-L", "POSTROUTING", "-n"])
        .output()
        .map(|out| {
            let output = String::from_utf8_lossy(&out.stdout);
            output.contains("MASQUERADE") || output.contains("SNAT")
        })
        .unwrap_or(false);
    checks.push(serde_json::json!({
        "name": "NAT Rules",
        "status": if nat_ok { "ok" } else { "info" },
        "message": if nat_ok { "NAT/Masquerade rules are configured" } else { "No NAT rules found (may be normal for some configurations)" }
    }));
    
    // Overall status
    let error_count = checks.iter().filter(|c| c["status"] == "error").count();
    let warning_count = checks.iter().filter(|c| c["status"] == "warning").count();
    
    let overall = if error_count > 0 {
        "error"
    } else if warning_count > 0 {
        "warning"
    } else {
        "ok"
    };
    
    HttpResponse::Ok().json(serde_json::json!({
        "peer_id": peer_id,
        "overall_status": overall,
        "checks": checks,
        "suggestions": if overall != "ok" {
            vec![
                if !wg_up { Some("Start the WireGuard tunnel from the Control Center") } else { None },
                if !handshake_ok { Some("Check if the peer is online and has correct endpoint configuration") } else { None },
                if !ip_forward { Some("Enable IP forwarding: sysctl -w net.ipv4.ip_forward=1") } else { None },
            ].into_iter().flatten().collect::<Vec<_>>()
        } else {
            vec![]
        }
    }))
}

// Password change endpoint
// Check if password is configured
#[get("/api/system/password/status")]
pub async fn get_password_status(req: HttpRequest) -> impl Responder {
    if let Err(e) = enforce_auth(req) {
        return e;
    }
    
    let config = match conf::util::get_config() {
        Ok(config) => config,
        Err(_) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Unable to get config"
            }));
        }
    };
    
    let has_password = !config.agent.web.password.hash.is_empty();
    
    HttpResponse::Ok().json(serde_json::json!({
        "has_password": has_password,
        "enabled": config.agent.web.password.enabled
    }))
}

#[derive(serde::Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: Option<String>,
    pub new_password: String,
}

#[post("/api/system/password")]
pub async fn post_change_password(req: HttpRequest, body: web::Json<ChangePasswordRequest>) -> impl Responder {
    if let Err(e) = enforce_auth(req) {
        return e;
    }
    
    // Get current config
    let mut config = match conf::util::get_config() {
        Ok(config) => config,
        Err(_) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Unable to get config"
            }));
        }
    };
    
    let has_existing_password = !config.agent.web.password.hash.is_empty();
    
    // If password exists, verify current password
    if has_existing_password {
        let current_password = match &body.current_password {
            Some(p) => p,
            None => {
                return HttpResponse::BadRequest().json(serde_json::json!({
                    "error": "Current password is required"
                }));
            }
        };
        
        let current_hash = match PasswordHash::new(&config.agent.web.password.hash) {
            Ok(hash) => hash,
            Err(e) => {
                log::error!("Invalid password hash format in configuration: {}", e);
                return HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "Server configuration error"
                }));
            }
        };
        
        if Argon2::default().verify_password(current_password.as_bytes(), &current_hash).is_err() {
            return HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "Current password is incorrect"
            }));
        }
    }
    
    // Validate new password
    let new_password = body.new_password.trim();
    if new_password.len() < 4 {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "New password must be at least 4 characters"
        }));
    }
    
    // Hash new password
    let mut sbytes = [0u8; 8];
    rand::rng().fill_bytes(&mut sbytes);
    let salt = match SaltString::encode_b64(&sbytes) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to generate salt: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to generate password salt"
            }));
        }
    };
    
    let new_hash = match Argon2::default().hash_password(new_password.as_bytes(), &salt) {
        Ok(hash) => hash.to_string(),
        Err(e) => {
            log::error!("Failed to hash password: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to hash new password"
            }));
        }
    };
    
    // Update config with new password hash and enable password auth
    config.agent.web.password.hash = new_hash;
    config.agent.web.password.enabled = true;
    
    if let Err(e) = conf::util::set_config(&mut config) {
        log::error!("Failed to save config: {}", e);
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "Failed to save new password"
        }));
    }
    
    let message = if has_existing_password {
        "Password changed successfully"
    } else {
        "Password set successfully"
    };
    
    log::info!("{}", message);
    HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": message
    }))
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
