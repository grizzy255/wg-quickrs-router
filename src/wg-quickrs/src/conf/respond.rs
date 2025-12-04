use crate::conf::util;
use crate::conf::network;
use crate::wireguard::cmd::sync_conf;
use crate::mode::mode::SystemMode;
use crate::mode::routing_pbr;
use wg_quickrs_lib::types::api::{SummaryDigest, ChangeSum};
use wg_quickrs_lib::validation::network::*;
use actix_web::{HttpResponse, web};
use chrono::{Duration, Utc};
use serde_json::json;
use uuid::Uuid;
use wg_quickrs_lib::helpers::remove_expired_reservations;
use wg_quickrs_lib::types::network::{ReservationData, NetworkWDigest};
use wg_quickrs_lib::types::config::ConfigFile;

macro_rules! get_mg_config_w_digest {
    () => {{
        util::CONFIG_W_NETWORK_DIGEST
            .get()
            .ok_or_else(|| HttpResponse::InternalServerError().body("internal config variables are not initialized"))?
            .write()
            .map_err(|_| HttpResponse::InternalServerError().body("unable to acquire lock on config variables"))?
    }};
}

macro_rules! post_mg_config_w_digest {
    ($c:expr) => {{
        let config_file = ConfigFile::from(&$c.to_config());
        $c.network_w_digest.network.updated_at = Utc::now();
        $c.network_w_digest = NetworkWDigest::try_from($c.network_w_digest.network.clone())
            .map_err(|_| HttpResponse::InternalServerError().body("unable to compute config digest"))?;

        let config_file_str = serde_yml::to_string(&config_file)
            .map_err(|_| HttpResponse::InternalServerError().body("unable to serialize config"))?;

        util::write_config(config_file_str)
            .map_err(|_| HttpResponse::InternalServerError().body("unable to write config"))?;
    }};
}

pub(crate) fn get_network_summary(query: web::Query<crate::web::api::SummaryBody>) -> Result<HttpResponse, HttpResponse> {
    let summary = util::get_summary()
        .map_err(|_| HttpResponse::InternalServerError().body("unable to get summary"))?;
    let response_data = if query.only_digest {
        json!(SummaryDigest::from(&summary))
    } else {
        json!(summary)
    };
    Ok(HttpResponse::Ok().json(response_data))
}

pub(crate) fn patch_network_config(body: web::Bytes) -> Result<HttpResponse, HttpResponse> {
    let body_raw = String::from_utf8_lossy(&body);
    let change_sum: ChangeSum = match serde_json::from_str(&body_raw) {
        Ok(val) => val,
        Err(err) => {
            log::error!("Failed to deserialize change_sum: {}", err);
            log::debug!("Request body: {}", body_raw);
            return Err(HttpResponse::BadRequest().body(format!("invalid JSON: {}", err)));
        }
    };

    log::debug!("update config with the change_sum = \n{:?}", change_sum);
    

    let mut c = get_mg_config_w_digest!();
    let this_peer_id = c.network_w_digest.network.this_peer;
    let mut changed_config = false;

    remove_expired_reservations(&mut c.network_w_digest.network);

    // process changed_fields
    if let Some(changed_fields) = &change_sum.changed_fields {
        if let Some(changed_fields_peers) = &changed_fields.peers {
            for (peer_id, peer_details) in changed_fields_peers {
                // Get router mode, interface name, and network clone before mutable borrow
                let is_router_mode = SystemMode::from(c.agent.router.mode.as_str()) == SystemMode::Router;
                let wg_interface = c.network_w_digest.network.name.clone();
                let network_for_validation = c.network_w_digest.network.clone();
                
                let mut network_copy = network_for_validation.clone();
                let mut old_address_opt = None;
                let mut new_address_opt = None;
                
                if let Some(peer_config) = c.network_w_digest.network.peers.get_mut(peer_id) {
                    if let Some(name) = &peer_details.name {
                        peer_config.name = parse_and_validate_peer_name(name).map_err(|e| {
                            HttpResponse::BadRequest().body(format!("changed_fields.peers.{}.name: {}", peer_id, e))
                        })?;
                    }
                    if let Some(address) = &peer_details.address {
                        old_address_opt = Some(peer_config.address.clone());
                        network_copy.peers.retain(|id, _| id != peer_id);
                        peer_config.address = validate_peer_address(address, &network_copy).map_err(|e| {
                            HttpResponse::BadRequest().body(format!("changed_fields.peers.{}.address: {}", peer_id, e))
                        })?;
                        new_address_opt = Some(peer_config.address.clone());
                    }
                    if let Some(endpoint) = &peer_details.endpoint {
                        peer_config.endpoint = validate_peer_endpoint(endpoint).map_err(|e| {
                            HttpResponse::BadRequest().body(format!("changed_fields.peers.{}.endpoint: {}", peer_id, e))
                        })?;
                    }
                    if let Some(kind) = &peer_details.kind {
                        peer_config.kind = parse_and_validate_peer_kind(kind).map_err(|e| {
                            HttpResponse::BadRequest().body(format!("changed_fields.peers.{}.kind: {}", peer_id, e))
                        })?;
                    }
                    if let Some(icon) = &peer_details.icon {
                        peer_config.icon = validate_peer_icon(icon).map_err(|e| {
                            HttpResponse::BadRequest().body(format!("changed_fields.peers.{}.icon: {}", peer_id, e))
                        })?;
                    }
                    if let Some(dns) = &peer_details.dns {
                        peer_config.dns = validate_peer_dns(dns).map_err(|e| {
                            HttpResponse::BadRequest().body(format!("changed_fields.peers.{}.dns: {}", peer_id, e))
                        })?;
                    }
                    if let Some(mtu) = &peer_details.mtu {
                        peer_config.mtu = validate_peer_mtu(mtu).map_err(|e| {
                            HttpResponse::BadRequest().body(format!("changed_fields.peers.{}.mtu: {}", peer_id, e))
                        })?;
                    }
                    if let Some(private_key) = &peer_details.private_key {
                        peer_config.private_key = *private_key;
                        // If deserialization succeeds, private_key is already validated.
                    }

                    if let Some(scripts) = &peer_details.scripts {
                        // Security check: prevent modifying scripts for this_peer
                        if *peer_id == this_peer_id {
                            return Err(HttpResponse::Forbidden().body("cannot modify scripts for this peer remotely"));
                        }

                        if let Some(scripts) = &scripts.pre_up {
                            peer_config.scripts.pre_up = validate_peer_scripts(scripts).map_err(|e| {
                                HttpResponse::BadRequest().body(format!("changed_fields.peers.{}.scripts.pre_up: {}", peer_id, e))
                            })?;
                        }
                        if let Some(scripts) = &scripts.post_up {
                            peer_config.scripts.post_up = validate_peer_scripts(scripts).map_err(|e| {
                                HttpResponse::BadRequest().body(format!("changed_fields.peers.{}.scripts.post_up: {}", peer_id, e))
                            })?;
                        }
                        if let Some(scripts) = &scripts.pre_down {
                            peer_config.scripts.pre_down = validate_peer_scripts(scripts).map_err(|e| {
                                HttpResponse::BadRequest().body(format!("changed_fields.peers.{}.scripts.pre_down: {}", peer_id, e))
                            })?;
                        }
                        if let Some(scripts) = &scripts.post_down {
                            peer_config.scripts.post_down = validate_peer_scripts(scripts).map_err(|e| {
                                HttpResponse::BadRequest().body(format!("changed_fields.peers.{}.scripts.post_down: {}", peer_id, e))
                            })?;
                        }
                    }
                    changed_config = true;
                } else {
                    return Err(HttpResponse::NotFound().body(format!("peer '{}' does not exist", peer_id)));
                }
                
                // STEP 4: Update routing table if peer IP changed and Router Mode is active
                // Skip the host peer - it shouldn't have a routing table
                // Do this after mutable borrow is released
                if *peer_id == this_peer_id {
                    log::debug!("Skipping routing table update for host peer {} after IP change", peer_id);
                } else if let (Some(old_address), Some(new_address)) = (old_address_opt, new_address_opt) {
                    log::debug!("Peer {} IP change detected: {} -> {}", peer_id, old_address, new_address);
                    if old_address != new_address && is_router_mode {
                        log::info!("Updating routing table for peer {} after IP change from {} to {}", peer_id, old_address, new_address);
                        let network_clone = c.network_w_digest.network.clone();
                        
                        // Update routes (update_peer_routes will create table if it doesn't exist and update PBR rules)
                        if let Err(e) = routing_pbr::update_peer_routes(peer_id, &network_clone, &wg_interface) {
                            log::warn!("Failed to update routes for peer {} after IP change: {}", peer_id, e);
                        } else {
                            log::info!("Updated routing table and PBR rules for peer {} after IP change from {} to {}", peer_id, old_address, new_address);
                        }
                    } else if old_address == new_address {
                        log::debug!("Peer {} IP unchanged ({}), skipping routing table update", peer_id, old_address);
                    } else if !is_router_mode {
                        log::debug!("Not in Router Mode, skipping routing table update for peer {}", peer_id);
                    }
                } else {
                    log::debug!("No IP change detected for peer {} (old: {:?}, new: {:?})", peer_id, old_address_opt, new_address_opt);
                }
            }
        }
        if let Some(changed_fields_connections) = &changed_fields.connections {
            for (connection_id, connection_details) in changed_fields_connections {
                if let Some(connection_config) =
                    c.network_w_digest.network.connections.get_mut(connection_id)
                {
                    if let Some(enabled) = connection_details.enabled {
                        connection_config.enabled = enabled;
                    }
                    if let Some(pre_shared_key) = connection_details.pre_shared_key {
                        connection_config.pre_shared_key = pre_shared_key;
                        // If deserialization succeeds, pre_shared_key is already validated.
                    }
                    if let Some(allowed_ips_a_to_b) = &connection_details.allowed_ips_a_to_b {
                        connection_config.allowed_ips_a_to_b = allowed_ips_a_to_b.clone();
                        // If deserialization succeeds, allowed_ips_a_to_b is already validated.
                    }
                    if let Some(allowed_ips_b_to_a) = &connection_details.allowed_ips_b_to_a {
                        connection_config.allowed_ips_b_to_a = allowed_ips_b_to_a.clone();
                        // If deserialization succeeds, allowed_ips_b_to_a is already validated.
                    }
                    if let Some(persistent_keepalive) = &connection_details.persistent_keepalive {
                        connection_config.persistent_keepalive = validate_conn_persistent_keepalive(persistent_keepalive).map_err(|e| {
                            HttpResponse::BadRequest().body(format!("changed_fields.connections.{}.persistent_keepalive: {}", connection_id, e))
                        })?;
                    }
                    changed_config = true;
                    
                    // STEP 4: Update routes for both peers if Router Mode is active and allowed_ips changed
                    // Check mode directly from config we already have (avoid deadlock)
                    if SystemMode::from(c.agent.router.mode.as_str()) == SystemMode::Router {
                        if connection_details.allowed_ips_a_to_b.is_some() || connection_details.allowed_ips_b_to_a.is_some() {
                            let wg_interface = &c.network_w_digest.network.name;
                            
                            // Update routes for peer A (skip if host peer)
                            if connection_id.a != this_peer_id {
                                if let Err(e) = routing_pbr::update_peer_routes(&connection_id.a, &c.network_w_digest.network, wg_interface) {
                                    log::warn!("Failed to update routes for peer {}: {}", connection_id.a, e);
                                }
                            } else {
                                log::debug!("Skipping routing table update for host peer {} in connection", connection_id.a);
                            }
                            
                            // Update routes for peer B (skip if host peer)
                            if connection_id.b != this_peer_id {
                                if let Err(e) = routing_pbr::update_peer_routes(&connection_id.b, &c.network_w_digest.network, wg_interface) {
                                    log::warn!("Failed to update routes for peer {}: {}", connection_id.b, e);
                                }
                            } else {
                                log::debug!("Skipping routing table update for host peer {} in connection", connection_id.b);
                            }
                        }
                    }
                } else {
                    return Err(HttpResponse::NotFound().body(format!("connection '{}' does not exist", connection_id)));
                }
            }
        }
    }

    // process added_peers
    if let Some(added_peers) = &change_sum.added_peers {
        log::debug!("Processing {} added peers", added_peers.len());
        for (peer_id, peer_details) in added_peers {
            log::debug!("Received peer {} with name: '{}', address: {}, endpoint: {:?}", 
                       peer_id, peer_details.name, peer_details.address, peer_details.endpoint);
            {
                if c.network_w_digest.network.peers.contains_key(peer_id) {
                    return Err(HttpResponse::Forbidden().body(format!("peer '{}' already exists", peer_id)));
                }
                if let Some(value) = c.network_w_digest.network.reservations.get(&peer_details.address)
                    && value.peer_id != *peer_id {
                    return Err(HttpResponse::Forbidden().body(format!("address '{}' is reserved for another peer_id", peer_details.address)));
                }
                // ensure the address is taken off the reservation list so check_internal_address succeeds (this won't be posted if it fails early)
                c.network_w_digest
                    .network
                    .reservations
                    .retain(|address, _|  *address != peer_details.address);

                // If deserialization succeeds, peer_id is already validated.
                log::debug!("Validating peer {} name: '{}' (length: {})", peer_id, peer_details.name, peer_details.name.len());
                if peer_details.name.is_empty() {
                    log::error!("Validation failed for peer {} name: peer name cannot be empty", peer_id);
                    return Err(HttpResponse::BadRequest().body(format!("added_peers.{}.name: peer name cannot be empty", peer_id)));
                }
                parse_and_validate_peer_name(&peer_details.name).map_err(|e| {
                    log::error!("Validation failed for peer {} name: {}", peer_id, e);
                    HttpResponse::BadRequest().body(format!("added_peers.{}.name: {}", peer_id, e))
                })?;
                log::debug!("Validating peer {} address: {}", peer_id, peer_details.address);
                validate_peer_address(&peer_details.address, &c.network_w_digest.network).map_err(|e| {
                    log::error!("Validation failed for peer {} address: {}", peer_id, e);
                    HttpResponse::BadRequest().body(format!("added_peers.{}.address: {}", peer_id, e))
                })?;
                log::debug!("Validating peer {} endpoint", peer_id);
                validate_peer_endpoint(&peer_details.endpoint).map_err(|e| {
                    log::error!("Validation failed for peer {} endpoint: {}", peer_id, e);
                    HttpResponse::BadRequest().body(format!("added_peers.{}.endpoint: {}", peer_id, e))
                })?;
                log::debug!("Validating peer {} kind: {}", peer_id, peer_details.kind);
                parse_and_validate_peer_kind(&peer_details.kind).map_err(|e| {
                    log::error!("Validation failed for peer {} kind: {}", peer_id, e);
                    HttpResponse::BadRequest().body(format!("added_peers.{}.kind: {}", peer_id, e))
                })?;
                log::debug!("Validating peer {} icon", peer_id);
                validate_peer_icon(&peer_details.icon).map_err(|e| {
                    log::error!("Validation failed for peer {} icon: {}", peer_id, e);
                    HttpResponse::BadRequest().body(format!("added_peers.{}.icon: {}", peer_id, e))
                })?;
                log::debug!("Validating peer {} dns", peer_id);
                validate_peer_dns(&peer_details.dns).map_err(|e| {
                    log::error!("Validation failed for peer {} dns: {}", peer_id, e);
                    HttpResponse::BadRequest().body(format!("added_peers.{}.dns: {}", peer_id, e))
                })?;
                // If deserialization succeeds, dns is already validated.
                log::debug!("Validating peer {} mtu", peer_id);
                validate_peer_mtu(&peer_details.mtu).map_err(|e| {
                    log::error!("Validation failed for peer {} mtu: {}", peer_id, e);
                    HttpResponse::BadRequest().body(format!("added_peers.{}.mtu: {}", peer_id, e))
                })?;
                // If private_key is provided and deserialization succeeds, it's already validated.
                // If not provided, a key will be auto-generated in Peer::from().
                log::debug!("Validating peer {} scripts", peer_id);
                validate_peer_scripts(&peer_details.scripts.pre_up).map_err(|e| {
                    log::error!("Validation failed for peer {} scripts.pre_up: {}", peer_id, e);
                    HttpResponse::BadRequest().body(format!("added_peers.{}.scripts.pre_up: {}", peer_id, e))
                })?;
                validate_peer_scripts(&peer_details.scripts.post_up).map_err(|e| {
                    log::error!("Validation failed for peer {} scripts.post_up: {}", peer_id, e);
                    HttpResponse::BadRequest().body(format!("added_peers.{}.scripts.post_up: {}", peer_id, e))
                })?;
                validate_peer_scripts(&peer_details.scripts.pre_down).map_err(|e| {
                    log::error!("Validation failed for peer {} scripts.pre_down: {}", peer_id, e);
                    HttpResponse::BadRequest().body(format!("added_peers.{}.scripts.pre_down: {}", peer_id, e))
                })?;
                validate_peer_scripts(&peer_details.scripts.post_down).map_err(|e| {
                    log::error!("Validation failed for peer {} scripts.post_down: {}", peer_id, e);
                    HttpResponse::BadRequest().body(format!("added_peers.{}.scripts.post_down: {}", peer_id, e))
                })?;
                log::debug!("All validations passed for peer {}, creating Peer object", peer_id);
                let mut added_peer = wg_quickrs_lib::types::network::Peer::from(peer_details);
                added_peer.created_at = Utc::now();
                added_peer.updated_at = added_peer.created_at;
                log::info!("Inserting peer {} into network with private_key present: {}", peer_id, added_peer.private_key.to_base64());
                c.network_w_digest.network.peers.insert(*peer_id, added_peer);
                changed_config = true;
                log::info!("Peer {} successfully inserted into network, changed_config = true", peer_id);
                
                // STEP 4: Create peer-specific routing table if Router Mode is active
                // Skip the host peer - it shouldn't have a routing table
                if *peer_id == this_peer_id {
                    log::debug!("[STEP4] Skipping routing table creation for host peer {}", peer_id);
                } else {
                    log::info!("[STEP4] Checking Router Mode for peer {}...", peer_id);
                    // Check mode directly from config we already have (avoid deadlock by calling get_current_mode)
                    let mode_str = c.agent.router.mode.as_str();
                    log::info!("[STEP4] Mode string from config: '{}'", mode_str);
                    let current_mode = SystemMode::from(mode_str);
                    log::info!("[STEP4] Parsed mode: {:?}", current_mode);
                    match current_mode {
                        SystemMode::Router => {
                            log::info!("[STEP4] Router Mode is active. Creating routing table for peer {}...", peer_id);
                            
                            // Get peer's advertised routes from connections
                            let routes = routing_pbr::get_peer_advertised_routes(peer_id, &c.network_w_digest.network);
                            log::info!("[STEP4] Found {} routes for peer {}: {:?}", routes.len(), peer_id, routes);
                            
                            // Create peer-specific routing table
                            log::debug!("[STEP4] Creating routing table for peer {}...", peer_id);
                            match routing_pbr::create_peer_routing_table(peer_id) {
                                Ok(table_id) => {
                                    log::info!("[STEP4] Created routing table {} for peer {}", table_id, peer_id);
                                    
                                    // Get WireGuard interface name
                                    let wg_interface = &c.network_w_digest.network.name;
                                    log::debug!("[STEP4] Installing {} routes into table {} for peer {} on interface {}", 
                                        routes.len(), table_id, peer_id, wg_interface);
                                    
                                // Install routes into peer's table
                                if let Err(e) = routing_pbr::install_peer_routes(
                                    peer_id,
                                    table_id,
                                    &routes,
                                    wg_interface,
                                ) {
                                    log::warn!("[STEP4] Failed to install routes for peer {}: {}", peer_id, e);
                                    // Don't fail the peer addition, but log the error
                                } else {
                                    log::info!("[STEP4] Successfully created routing table {} and installed {} routes for peer {}", 
                                        table_id, routes.len(), peer_id);
                                }
                                
                                // Install PBR rules for this peer
                                let lan_interface = routing_pbr::find_lan_interface()
                                    .unwrap_or_else(|_| "eth0".to_string());
                                if let Err(e) = routing_pbr::install_pbr_rules_for_peer(peer_id, table_id, &routes, &lan_interface) {
                                    log::warn!("[STEP4] Failed to install PBR rules for peer {}: {}", peer_id, e);
                                } else {
                                    log::info!("[STEP4] Successfully installed PBR rules for peer {}", peer_id);
                                }
                                
                                // If this peer has default route and no exit node is set, set it as exit node
                                if (routes.contains(&"0.0.0.0/0".to_string()) || routes.contains(&"default".to_string())) 
                                    && routing_pbr::get_exit_node().unwrap_or(None).is_none() {
                                    log::info!("[STEP4] Setting peer {} as exit node (first peer with default route)", peer_id);
                                    if let Err(e) = routing_pbr::set_exit_node(peer_id, Some(&c.network_w_digest.network)) {
                                        log::warn!("[STEP4] Failed to set exit node: {}", e);
                                    }
                                }
                                }
                                Err(e) => {
                                    log::warn!("[STEP4] Failed to create routing table for peer {}: {}", peer_id, e);
                                }
                            }
                        }
                        SystemMode::Host => {
                            log::info!("[STEP4] Host Mode is active. Skipping routing table creation for peer {}", peer_id);
                        }
                    }
                }
            }
        }
    }

    // process removed_peers
    if let Some(removed_peers) = &change_sum.removed_peers {
        for peer_id in removed_peers {
            {
                if *peer_id == this_peer_id {
                    return Err(HttpResponse::Forbidden().body("cannot remove this peer"));
                }
                
                // STEP 4: Remove peer-specific routing table if Router Mode is active
                // Check mode directly from config we already have (avoid deadlock)
                if SystemMode::from(c.agent.router.mode.as_str()) == SystemMode::Router {
                    // Get the table_id for this peer
                    if let Ok(Some(table_id)) = routing_pbr::get_peer_table_id(peer_id) {
                        // Remove peer's routing table (pass network to avoid deadlock)
                        if let Err(e) = routing_pbr::remove_peer_routing_table_impl(peer_id, table_id, Some(&c.network_w_digest.network)) {
                            log::warn!("Failed to remove routing table for peer {}: {}", peer_id, e);
                        } else {
                            log::info!("Successfully removed routing table {} for peer {}", table_id, peer_id);
                        }
                    }
                }
                
                c.network_w_digest.network.peers.remove(peer_id);
                // automatically remove connections
                for connection_id in c.network_w_digest.network.connections.clone().keys().filter(|&x| x.contains(peer_id)) {
                    c.network_w_digest.network.connections.remove(connection_id);
                }
                changed_config = true;
            }
        }
    }

    // process added_connections
    if let Some(added_connections) = &change_sum.added_connections {
        for (connection_id, connection_details) in added_connections {
            {
                if !c.network_w_digest.network.peers.contains_key(&connection_id.a) {
                    return Err(HttpResponse::BadRequest().body(format!("added_connections.{}: 'peer_id' does not exist", connection_id.a)));
                }
                if !c.network_w_digest.network.peers.contains_key(&connection_id.b) {
                    return Err(HttpResponse::BadRequest().body(format!("added_connections.{}: 'peer_id' does not exist", connection_id.b)));
                }
                if c.network_w_digest.network.connections.contains_key(connection_id) {
                    return Err(HttpResponse::Forbidden().body(format!("connection '{}' already exists", connection_id)));
                }
                if connection_id.a == connection_id.b {
                    return Err(HttpResponse::Forbidden().body(format!("loopback connection detected: {}", connection_id)));
                }

                // If deserialization succeeds, pre_shared_key is already validated.
                // If deserialization succeeds, allowed_ips_a_to_b is already validated.
                // If deserialization succeeds, allowed_ips_b_to_a is already validated.
                validate_conn_persistent_keepalive(&connection_details.persistent_keepalive).map_err(|e| {
                    HttpResponse::BadRequest().body(format!("added_connections.{}.persistent_keepalive: {}", connection_id, e))
                })?;

                c.network_w_digest
                    .network
                    .connections
                    .insert(connection_id.clone(), connection_details.clone());
                changed_config = true;
                
                // STEP 4: Update routes for both peers if Router Mode is active
                // Check mode directly from config we already have (avoid deadlock)
                if SystemMode::from(c.agent.router.mode.as_str()) == SystemMode::Router {
                    let wg_interface = &c.network_w_digest.network.name;
                    
                    // Update routes for peer A (create table if needed, skip if host peer)
                    // update_peer_routes will also update PBR rules
                    if connection_id.a != this_peer_id {
                        if let Err(e) = routing_pbr::update_peer_routes(&connection_id.a, &c.network_w_digest.network, wg_interface) {
                            log::warn!("Failed to update routes for peer {}: {}", connection_id.a, e);
                        }
                    } else {
                        log::debug!("Skipping routing table creation/update for host peer {} in connection", connection_id.a);
                    }
                    
                    // Update routes for peer B (create table if needed, skip if host peer)
                    // update_peer_routes will also update PBR rules
                    if connection_id.b != this_peer_id {
                        if let Err(e) = routing_pbr::update_peer_routes(&connection_id.b, &c.network_w_digest.network, wg_interface) {
                            log::warn!("Failed to update routes for peer {}: {}", connection_id.b, e);
                        }
                    } else {
                        log::debug!("Skipping routing table creation/update for host peer {} in connection", connection_id.b);
                    }
                }
            }
        }
    }

    // process removed_connections
    if let Some(removed_connections) = &change_sum.removed_connections {
        for connection_id in removed_connections {
            {
                // STEP 4: Update routes for both peers if Router Mode is active
                // Check mode directly from config we already have (avoid deadlock)
                if SystemMode::from(c.agent.router.mode.as_str()) == SystemMode::Router {
                    let wg_interface = &c.network_w_digest.network.name;
                    
                    // Update routes for peer A (remove routes from this connection, skip if host peer)
                    if connection_id.a != this_peer_id {
                        if let Err(e) = routing_pbr::update_peer_routes(&connection_id.a, &c.network_w_digest.network, wg_interface) {
                            log::warn!("Failed to update routes for peer {}: {}", connection_id.a, e);
                        }
                    } else {
                        log::debug!("Skipping routing table update for host peer {} in connection removal", connection_id.a);
                    }
                    
                    // Update routes for peer B (remove routes from this connection, skip if host peer)
                    if connection_id.b != this_peer_id {
                        if let Err(e) = routing_pbr::update_peer_routes(&connection_id.b, &c.network_w_digest.network, wg_interface) {
                            log::warn!("Failed to update routes for peer {}: {}", connection_id.b, e);
                        }
                    } else {
                        log::debug!("Skipping routing table update for host peer {} in connection removal", connection_id.b);
                    }
                }
                
                c.network_w_digest.network.connections.remove(connection_id);
                changed_config = true;
            }
        }
    }
    if !changed_config {
        log::debug!("nothing to update");
        return Err(HttpResponse::BadRequest().body("nothing to update"));
    }
    log::info!("Saving config with changed_config = true");
    post_mg_config_w_digest!(c);
    log::info!("config updated successfully");

    if c.agent.vpn.enabled {
        sync_conf(&c.clone().to_config()).map_err(|e| {
            log::error!("{e}");
            HttpResponse::InternalServerError().body("unable to synchronize config")
        })?;
    }

    Ok(HttpResponse::Ok().json(json!(change_sum)))
}

pub(crate) fn post_network_reserve_address() -> Result<HttpResponse, HttpResponse> {
    let mut c = get_mg_config_w_digest!();
    remove_expired_reservations(&mut c.network_w_digest.network);
    let next_address = network::get_next_available_address(&c.network_w_digest.network)
        .ok_or_else(|| HttpResponse::Conflict().body("No more IP addresses available in the pool".to_string()))?;

    let reservation_peer_id = Uuid::new_v4();
    let reservation_valid_until = Utc::now() + Duration::minutes(10);
    c.network_w_digest.network.reservations.insert(next_address, ReservationData {
        peer_id: reservation_peer_id,
        valid_until: reservation_valid_until,
    });
    post_mg_config_w_digest!(c);
    log::info!("reserved address {} for {} until {}", next_address, reservation_peer_id, reservation_valid_until);
    
    Ok(HttpResponse::Ok().json(json!({
        "address": next_address,
        "peer_id": reservation_peer_id,
        "valid_until": reservation_valid_until
    })))
}
