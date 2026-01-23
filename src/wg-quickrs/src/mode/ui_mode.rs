// UI Mode: REST endpoints for mode toggle, modals, interactive map actions
// Prepares messages for frontend (e.g., "Cannot Change Mode")
// 
// Responsibilities:
// - STEP 1: UI endpoints for mode toggle
// - STEP 3: Modal handling for mode switch restrictions
// - STEP 4: Default mode behavior endpoints
// - STEP 6: Last-peer deletion prompt handling
// - STEP 7: Interactive map actions for active/backup peer selection

use actix_web::{HttpRequest, HttpResponse};
use crate::conf;
use wg_quickrs_lib::types::network::{EndpointAddress, Network};
use wg_quickrs_lib::helpers::wg_public_key_from_private_key;
use uuid::Uuid;

/// Helper function to format EndpointAddress for display
fn format_endpoint_address(addr: &EndpointAddress) -> String {
    match addr {
        EndpointAddress::None => "none".to_string(),
        EndpointAddress::Ipv4AndPort(ip_port) => format!("{}:{}", ip_port.ipv4, ip_port.port),
        EndpointAddress::HostnameAndPort(host_port) => format!("{}:{}", host_port.hostname, host_port.port),
    }
}

/// WireGuard parameters for a peer, extracted from conf.yml
/// Used for start/reconnect operations to ensure all values come from config
#[derive(Debug)]
struct PeerWgParams {
    public_key: String,
    preshared_key: Option<String>,
    endpoint: Option<String>,
    allowed_ips: Vec<String>,
    persistent_keepalive: Option<u16>,
}

/// Extract WireGuard parameters for a peer from the network config
/// This ensures we use the same source of truth as the initial config generation
fn get_peer_wg_params(network: &Network, peer_id: &Uuid, include_default_route: bool) -> Option<PeerWgParams> {
    let this_peer_id = network.this_peer;
    
    // Get peer details
    let peer = network.peers.get(peer_id)?;
    
    // Get public key from private key
    let public_key = wg_public_key_from_private_key(&peer.private_key).to_base64();
    
    // Find the connection between this router and the target peer
    let mut preshared_key: Option<String> = None;
    let mut allowed_ips: Vec<String> = vec![format!("{}/32", peer.address)];
    let mut persistent_keepalive: Option<u16> = None;
    
    for (conn_id, conn_details) in &network.connections {
        if !conn_id.contains(peer_id) || !conn_id.contains(&this_peer_id) {
            continue;
        }
        if !conn_details.enabled {
            continue;
        }
        
        // Get preshared key
        preshared_key = Some(conn_details.pre_shared_key.to_base64());
        
        // Get persistent keepalive
        if conn_details.persistent_keepalive.enabled {
            persistent_keepalive = Some(conn_details.persistent_keepalive.period);
        }
        
        // Get allowed IPs (from this router's perspective, what IPs can reach the peer)
        let ips = if conn_id.a == *peer_id {
            &conn_details.allowed_ips_a_to_b
        } else {
            &conn_details.allowed_ips_b_to_a
        };
        
        for ip in ips {
            let ip_str = ip.to_string();
            // Skip 0.0.0.0/0 unless explicitly requested (exit node)
            if ip_str == "0.0.0.0/0" || ip_str == "default" {
                if include_default_route {
                    if !allowed_ips.contains(&"0.0.0.0/0".to_string()) {
                        allowed_ips.push("0.0.0.0/0".to_string());
                    }
                }
            } else if !allowed_ips.contains(&ip_str) {
                allowed_ips.push(ip_str);
            }
        }
        break;
    }
    
    // Get endpoint if enabled
    let endpoint = if peer.endpoint.enabled {
        let ep = format_endpoint_address(&peer.endpoint.address);
        if !ep.is_empty() && ep != "none" {
            Some(ep)
        } else {
            None
        }
    } else {
        None
    };
    
    Some(PeerWgParams {
        public_key,
        preshared_key,
        endpoint,
        allowed_ips,
        persistent_keepalive,
    })
}

// Get current mode (Host or Router) from config
pub async fn get_mode(_req: HttpRequest) -> HttpResponse {
    match conf::util::get_config() {
        Ok(config) => {
            HttpResponse::Ok().json(serde_json::json!({
                "mode": config.agent.router.mode,
                "lan_cidr": config.agent.router.lan_cidr
            }))
        }
        Err(_) => {
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to load config"
            }))
        }
    }
}

// Toggle between Host and Router Mode
pub async fn toggle_mode(_req: HttpRequest, body: actix_web::web::Bytes) -> HttpResponse {
    use crate::mode::mode::{switch_mode, SystemMode};
    use serde_json::Value;
    
    // Parse request body
    let body_str = match String::from_utf8(body.to_vec()) {
        Ok(s) => s,
        Err(e) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": format!("Invalid request body: {}", e)
            }));
        }
    };
    
    let json: Value = match serde_json::from_str(&body_str) {
        Ok(v) => v,
        Err(e) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": format!("Invalid JSON: {}", e)
            }));
        }
    };
    
    // Extract mode and lan_cidr
    let mode_str = json.get("mode")
        .and_then(|v| v.as_str())
        .unwrap_or("host");
    
    let target_mode = SystemMode::from(mode_str);
    let lan_cidr = json.get("lan_cidr")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    
    // Switch mode
    match switch_mode(target_mode, lan_cidr) {
        Ok(_) => {
            // Get updated config to return current state
            match conf::util::get_config() {
                Ok(config) => {
                    HttpResponse::Ok().json(serde_json::json!({
                        "mode": config.agent.router.mode,
                        "lan_cidr": config.agent.router.lan_cidr
                    }))
                }
                Err(_) => {
                    HttpResponse::Ok().json(serde_json::json!({
                        "mode": mode_str,
                        "lan_cidr": json.get("lan_cidr").and_then(|v| v.as_str())
                    }))
                }
            }
        }
        Err(e) => {
            // Check if it's a PeersExist error and return appropriate status code
            let status = if e.to_string().contains("peers are configured") {
                actix_web::http::StatusCode::FORBIDDEN
            } else {
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR
            };
            
            HttpResponse::build(status).json(serde_json::json!({
                "error": e.to_string()
            }))
        }
    }
}

// Get mode switch restrictions status
pub async fn can_switch_mode(_req: HttpRequest) -> HttpResponse {
    use crate::mode::mode::can_switch_mode;
    
    match can_switch_mode() {
        Ok(can_switch) => {
            if can_switch {
                HttpResponse::Ok().json(serde_json::json!({
                    "can_switch": true,
                    "reason": null
                }))
            } else {
                HttpResponse::Ok().json(serde_json::json!({
                    "can_switch": false,
                    "reason": "Cannot switch mode: peers are configured"
                }))
            }
        }
        Err(e) => {
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": e.to_string()
            }))
        }
    }
}

// Update peer route status (active/backup for overlapping routes)
// Currently supports setting exit node for default route (0.0.0.0/0)
pub async fn update_peer_route_status(_req: HttpRequest, body: actix_web::web::Bytes) -> HttpResponse {
    // Parse request body
    let body_str = match String::from_utf8(body.to_vec()) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to parse request body: {}", e);
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Invalid request body"
            }));
        }
    };
    
    let request: serde_json::Value = match serde_json::from_str(&body_str) {
        Ok(v) => v,
        Err(e) => {
            log::error!("Failed to parse JSON: {}", e);
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": format!("Invalid JSON: {}", e)
            }));
        }
    };
    
    let prefix = match request.get("prefix") {
        Some(p) => match p.as_str() {
            Some(s) => s,
            None => {
                return HttpResponse::BadRequest().json(serde_json::json!({
                    "error": "prefix must be a string"
                }));
            }
        },
        None => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "prefix is required"
            }));
        }
    };
    
    let active_peer_id = match request.get("active_peer_id") {
        Some(p) => match p.as_str() {
            Some(s) => s,
            None => {
                return HttpResponse::BadRequest().json(serde_json::json!({
                    "error": "active_peer_id must be a string"
                }));
            }
        },
        None => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "active_peer_id is required"
            }));
        }
    };
    
    let backup_peer_ids = match request.get("backup_peer_ids") {
        Some(b) => match b.as_array() {
            Some(arr) => arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<String>>(),
            None => Vec::new(),
        },
        None => Vec::new(),
    };
    
    log::info!("Updating peer route status: prefix={}, active_peer_id={}, backup_peer_ids={:?}", 
        prefix, active_peer_id, backup_peer_ids);
    
    // Currently only support default route (0.0.0.0/0)
    if prefix == "0.0.0.0/0" || prefix == "default" {
        // Parse peer ID
        let peer_uuid = match uuid::Uuid::parse_str(active_peer_id) {
            Ok(u) => u,
            Err(e) => {
                log::error!("Invalid peer ID: {}", e);
                return HttpResponse::BadRequest().json(serde_json::json!({
                    "error": format!("Invalid peer ID: {}", e)
                }));
            }
        };
        
        // Set exit node (load config if needed - API call doesn't hold lock)
        match super::routing_pbr::set_exit_node(&peer_uuid, None) {
            Ok(_) => {
                log::info!("Successfully set peer {} as exit node", active_peer_id);
                
                // Bug 4 fix: Clear primary_exit_node on manual gateway switch
                // This prevents automatic fail-back to a stale primary after user manually changes gateway
                if let Err(e) = super::routing_pbr::set_primary_exit_node(None) {
                    log::warn!("Failed to clear primary exit node after manual switch: {}", e);
                } else {
                    log::debug!("Cleared primary exit node after manual gateway switch");
                }
                
                HttpResponse::Ok().json(serde_json::json!({
                    "success": true,
                    "message": format!("Set peer {} as exit node for default route", active_peer_id)
                }))
            }
            Err(e) => {
                log::error!("Failed to set exit node: {}", e);
                HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": format!("Failed to set exit node: {}", e)
                }))
            }
        }
    } else {
        // For other prefixes, use the generic function (future implementation)
        match super::routing_pbr::set_active_peer_for_prefix(prefix, active_peer_id, &backup_peer_ids) {
            Ok(_) => {
                HttpResponse::Ok().json(serde_json::json!({
                    "success": true,
                    "message": format!("Set peer {} as active for prefix {}", active_peer_id, prefix)
                }))
            }
            Err(e) => {
                log::error!("Failed to set active peer: {}", e);
                HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": format!("Failed to set active peer: {}", e)
                }))
            }
        }
    }
}

// Peer control actions: stop, start, reconnect
pub async fn peer_control(_req: HttpRequest, body: actix_web::web::Bytes) -> HttpResponse {
    use crate::helpers::shell_cmd;
    
    // Parse request body
    let body_str = match String::from_utf8(body.to_vec()) {
        Ok(s) => s,
        Err(e) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": format!("Invalid request body: {}", e)
            }));
        }
    };
    
    let request: serde_json::Value = match serde_json::from_str(&body_str) {
        Ok(v) => v,
        Err(e) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": format!("Invalid JSON: {}", e)
            }));
        }
    };
    
    let peer_id = match request.get("peer_id").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "peer_id is required"
            }));
        }
    };
    
    let action = match request.get("action").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "action is required (stop, start, reconnect)"
            }));
        }
    };
    
    // Parse peer UUID
    let peer_uuid = match Uuid::parse_str(peer_id) {
        Ok(u) => u,
        Err(e) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": format!("Invalid peer ID: {}", e)
            }));
        }
    };
    
    // Get config
    let config = match conf::util::get_config() {
        Ok(c) => c,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to load config: {}", e)
            }));
        }
    };
    
    let wg_interface = &config.network.name;
    
    // Check if this peer is the active exit node (include default route if so)
    let is_exit_node = match super::routing_pbr::get_exit_node() {
        Ok(Some(exit_id)) => exit_id == peer_uuid,
        _ => false,
    };
    
    // Get WireGuard parameters from conf.yml using helper function
    let wg_params = match get_peer_wg_params(&config.network, &peer_uuid, is_exit_node) {
        Some(p) => p,
        None => {
            return HttpResponse::NotFound().json(serde_json::json!({
                "error": format!("Peer {} not found or no connection configured", peer_id)
            }));
        }
    };
    
    log::debug!("Peer {} WG params from conf.yml: {:?}", peer_id, wg_params);
    
    match action {
        "stop" => {
            // Remove peer from WireGuard interface
            log::info!("Stopping peer {} ({})", peer_id, wg_params.public_key);
            
            match shell_cmd(&["wg", "set", wg_interface, "peer", &wg_params.public_key, "remove"]) {
                Ok(_) => {
                    log::info!("Successfully stopped peer {}", peer_id);
                    HttpResponse::Ok().json(serde_json::json!({
                        "success": true,
                        "message": format!("Stopped peer {}", peer_id),
                        "was_exit_node": is_exit_node
                    }))
                }
                Err(e) => {
                    log::error!("Failed to stop peer {}: {}", peer_id, e);
                    HttpResponse::InternalServerError().json(serde_json::json!({
                        "error": format!("Failed to stop peer: {}", e)
                    }))
                }
            }
        }
        "start" | "reconnect" => {
            // For reconnect, first remove the peer
            if action == "reconnect" {
                log::info!("Reconnecting peer {} ({})", peer_id, wg_params.public_key);
                if let Err(e) = shell_cmd(&["wg", "set", wg_interface, "peer", &wg_params.public_key, "remove"]) {
                    log::error!("Failed to remove peer {} during reconnect: {}", peer_id, e);
                    return HttpResponse::InternalServerError().json(serde_json::json!({
                        "error": format!("Failed to remove peer during reconnect: {}", e)
                    }));
                }
                // Small delay to ensure removal is complete
                std::thread::sleep(std::time::Duration::from_millis(100));
            } else {
                log::info!("Starting peer {} ({})", peer_id, wg_params.public_key);
            }
            
            // Write preshared key to temp file (wg set requires file input)
            let psk_tempfile = if let Some(ref psk) = wg_params.preshared_key {
                use std::io::Write;
                match tempfile::NamedTempFile::new() {
                    Ok(mut f) => {
                        if let Err(e) = f.write_all(psk.as_bytes()) {
                            log::warn!("Failed to write preshared key to temp file: {}", e);
                            None
                        } else {
                            Some(f)
                        }
                    }
                    Err(e) => {
                        log::warn!("Failed to create temp file for preshared key: {}", e);
                        None
                    }
                }
            } else {
                None
            };
            
            // Build allowed-ips string
            let allowed_ips_str = wg_params.allowed_ips.join(",");
            
            // Build command with all parameters from conf.yml
            let mut cmd: Vec<&str> = vec!["wg", "set", wg_interface, "peer", &wg_params.public_key, "allowed-ips", &allowed_ips_str];
            
            // Add endpoint if configured
            let endpoint_ref: String;
            if let Some(ref ep) = wg_params.endpoint {
                endpoint_ref = ep.clone();
                cmd.push("endpoint");
                cmd.push(&endpoint_ref);
            }
            
            // Add preshared key if available
            let psk_path_str: String;
            if let Some(ref psk_file) = psk_tempfile {
                psk_path_str = psk_file.path().to_string_lossy().to_string();
                cmd.push("preshared-key");
                cmd.push(&psk_path_str);
            }
            
            // Add persistent keepalive if configured
            let keepalive_str: String;
            if let Some(period) = wg_params.persistent_keepalive {
                keepalive_str = period.to_string();
                cmd.push("persistent-keepalive");
                cmd.push(&keepalive_str);
            }
            
            let action_past = if action == "reconnect" { "reconnected" } else { "started" };
            
            match shell_cmd(&cmd) {
                Ok(_) => {
                    log::info!("Successfully {} peer {} with allowed-ips: {}, psk: {}, endpoint: {:?}, keepalive: {:?}", 
                              action_past, peer_id, allowed_ips_str, wg_params.preshared_key.is_some(), 
                              wg_params.endpoint, wg_params.persistent_keepalive);
                    
                    // After starting any peer, check if there's a persisted exit node and restore its routing
                    // This ensures exit node routing is restored even if we're starting a different peer
                    match super::routing_pbr::get_exit_node() {
                        Ok(Some(saved_exit_node_id)) => {
                            log::info!("Found persisted exit node {}, restoring exit node routing...", saved_exit_node_id);
                            if let Err(e) = super::routing_pbr::set_exit_node(&saved_exit_node_id, Some(&config.network)) {
                                log::warn!("Failed to restore exit node routing for {}: {} (peer {} was started successfully)", 
                                          saved_exit_node_id, e, peer_id);
                                // Don't fail the whole operation - peer is started, routing can be fixed manually
                            } else {
                                log::info!("Successfully restored exit node routing for {}", saved_exit_node_id);
                            }
                        }
                        Ok(None) => {
                            log::debug!("No persisted exit node found, skipping exit node routing restoration");
                        }
                        Err(e) => {
                            log::warn!("Failed to check persisted exit node: {} (peer {} was started successfully)", e, peer_id);
                        }
                    }
                    
                    HttpResponse::Ok().json(serde_json::json!({
                        "success": true,
                        "message": format!("{} peer {}", action_past.chars().next().unwrap().to_uppercase().to_string() + &action_past[1..], peer_id)
                    }))
                }
                Err(e) => {
                    log::error!("Failed to {} peer {}: {}", action, peer_id, e);
                    HttpResponse::InternalServerError().json(serde_json::json!({
                        "error": format!("Failed to {} peer: {}", action, e)
                    }))
                }
            }
        }
        _ => {
            HttpResponse::BadRequest().json(serde_json::json!({
                "error": format!("Invalid action: {}. Valid actions are: stop, start, reconnect", action)
            }))
        }
    }
}

// Get exit node information (current exit node and peers with default routes)
pub async fn get_exit_node_info(_req: HttpRequest) -> HttpResponse {
    use crate::mode::routing_pbr::{get_exit_node, get_peers_with_default_route, get_exit_node_health};
    
    // Get current config to check mode and get network info
    let config = match conf::util::get_config() {
        Ok(c) => c,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to load config: {}", e)
            }));
        }
    };
    
    // Only return exit node info if in Router Mode
    if config.agent.router.mode.as_str() != "router" {
        return HttpResponse::Ok().json(serde_json::json!({
            "exit_node": null,
            "peers_with_default_route": [],
            "health_status": []
        }));
    }
    
    // Get all peers with default routes first (needed for both exit node and health)
    let peers_with_default = get_peers_with_default_route(&config.network);
    let peers_with_default_str: Vec<String> = peers_with_default
        .iter()
        .map(|id| id.to_string())
        .collect();
    
    // Get current exit node - optimize by loading state once and reusing
    // get_exit_node() loads state, but get_exit_node_health() also loads it
    // For now, keep separate loads but they're fast (file read)
    let exit_node = match get_exit_node() {
        Ok(Some(id)) => Some(id.to_string()),
        Ok(None) => None,
        Err(e) => {
            log::warn!("Failed to get exit node: {}", e);
            None
        }
    };
    
    // Get health status for exit nodes
    let wg_interface = &config.network.name;
    let health_status = get_exit_node_health(&config.network, wg_interface);
    let health_json: Vec<serde_json::Value> = health_status.iter().map(|h| {
        serde_json::json!({
            "peer_id": h.peer_id.to_string(),
            "packet_loss_percent": h.packet_loss_percent,
            "jitter_ms": h.jitter_ms,
            "is_online": h.is_online,
            "last_handshake": h.last_handshake,
            "first_handshake": h.first_handshake,
            "latency_ms": h.latency_ms,
            "transfer_rx": h.transfer_rx,
            "transfer_tx": h.transfer_tx,
            "endpoint": h.endpoint
        })
    }).collect();
    
    // Get auto-failover status
    let auto_failover = super::routing_pbr::get_auto_failover().unwrap_or(false);
    
    HttpResponse::Ok().json(serde_json::json!({
        "exit_node": exit_node,
        "peers_with_default_route": peers_with_default_str,
        "health_status": health_json,
        "auto_failover": auto_failover
    }))
}

/// Toggle LAN access for a specific peer
pub async fn set_peer_lan_access(_req: HttpRequest, body: actix_web::web::Bytes) -> HttpResponse {
    use crate::mode::routing_pbr;
    
    #[derive(serde::Deserialize)]
    struct LanAccessRequest {
        peer_id: String,
        has_lan_access: bool,
    }
    
    let request: LanAccessRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": format!("Invalid request body: {}", e)
            }));
        }
    };
    
    // Parse peer ID
    let peer_id = match Uuid::parse_str(&request.peer_id) {
        Ok(id) => id,
        Err(e) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": format!("Invalid peer ID: {}", e)
            }));
        }
    };
    
    // Get current config
    let config = match conf::util::get_config() {
        Ok(c) => c,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to load config: {}", e)
            }));
        }
    };
    
    // Check if we're in router mode
    if config.agent.router.mode.as_str() != "router" {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "LAN access control is only available in Router Mode"
        }));
    }
    
    // Verify peer exists
    if !config.network.peers.contains_key(&peer_id) {
        return HttpResponse::NotFound().json(serde_json::json!({
            "error": format!("Peer {} not found", request.peer_id)
        }));
    }
    
    // Don't allow changing LAN access for the router itself
    if peer_id == config.network.this_peer {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Cannot change LAN access for the router itself"
        }));
    }
    
    // Update the LAN access setting
    match routing_pbr::set_peer_lan_access(&peer_id, request.has_lan_access, &config.network) {
        Ok(new_state) => {
            let peer_name = config.network.peers.get(&peer_id)
                .map(|p| p.name.clone())
                .unwrap_or_else(|| request.peer_id.clone());
            
            log::info!("Updated LAN access for peer {} ({}): {}", peer_name, request.peer_id, new_state);
            
            HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "peer_id": request.peer_id,
                "has_lan_access": new_state,
                "message": format!("LAN access {} for {}", if new_state { "enabled" } else { "disabled" }, peer_name)
            }))
        }
        Err(e) => {
            log::error!("Failed to update LAN access for peer {}: {}", request.peer_id, e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to update LAN access: {}", e)
            }))
        }
    }
}

/// Get LAN access status for all peers
pub async fn get_peer_lan_access_all(_req: HttpRequest) -> HttpResponse {
    use crate::mode::routing_pbr;
    
    // Get current config
    let config = match conf::util::get_config() {
        Ok(c) => c,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to load config: {}", e)
            }));
        }
    };
    
    // Check if we're in router mode
    if config.agent.router.mode.as_str() != "router" {
        return HttpResponse::Ok().json(serde_json::json!({
            "peer_lan_access": {}
        }));
    }
    
    // Get all LAN access settings
    match routing_pbr::get_all_peer_lan_access() {
        Ok(lan_access_map) => {
            // Build response with peer names
            let mut result: std::collections::HashMap<String, serde_json::Value> = std::collections::HashMap::new();
            
            for (peer_id, peer) in &config.network.peers {
                if *peer_id == config.network.this_peer {
                    continue; // Skip router itself
                }
                let peer_id_str = peer_id.to_string();
                let has_access = lan_access_map.get(&peer_id_str).copied().unwrap_or(true);
                result.insert(peer_id_str, serde_json::json!({
                    "name": peer.name,
                    "has_lan_access": has_access
                }));
            }
            
            HttpResponse::Ok().json(serde_json::json!({
                "peer_lan_access": result
            }))
        }
        Err(e) => {
            log::error!("Failed to get LAN access settings: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to get LAN access settings: {}", e)
            }))
        }
    }
}

/// Get Smart Gateway (auto-failover) status
pub async fn get_auto_failover(_req: HttpRequest) -> HttpResponse {
    use crate::mode::routing_pbr;
    
    match routing_pbr::get_auto_failover() {
        Ok(enabled) => {
            HttpResponse::Ok().json(serde_json::json!({
                "enabled": enabled
            }))
        }
        Err(e) => {
            log::error!("Failed to get auto-failover status: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to get auto-failover status: {}", e)
            }))
        }
    }
}

/// Set Smart Gateway (auto-failover) status
pub async fn set_auto_failover(_req: HttpRequest, body: actix_web::web::Bytes) -> HttpResponse {
    use crate::mode::routing_pbr;
    
    #[derive(serde::Deserialize)]
    struct AutoFailoverRequest {
        enabled: bool,
    }
    
    let request: AutoFailoverRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": format!("Invalid request body: {}", e)
            }));
        }
    };
    
    // Get current config to check mode
    let config = match conf::util::get_config() {
        Ok(c) => c,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to load config: {}", e)
            }));
        }
    };
    
    // Only allow in router mode
    if config.agent.router.mode.as_str() != "router" {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Smart Gateway is only available in Router Mode"
        }));
    }
    
    match routing_pbr::set_auto_failover(request.enabled) {
        Ok(_) => {
            log::info!("Smart Gateway (auto-failover) {}", if request.enabled { "enabled" } else { "disabled" });
            HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "enabled": request.enabled,
                "message": format!("Smart Gateway {}", if request.enabled { "enabled" } else { "disabled" })
            }))
        }
        Err(e) => {
            log::error!("Failed to set auto-failover: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to set auto-failover: {}", e)
            }))
        }
    }
}

