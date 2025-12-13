// Core mode switching logic
// Prevents switch when peers exist; orchestrates enabling/disabling forwarding, persisting mode state
//
// Responsibilities:
// - STEP 2: Mode switching logic (enable forwarding, persist state)
// - STEP 3: Prevent switch when peers exist
// - STEP 4: Default mode behavior
// - STEP 6: Handle last peer deletion

use super::persist::{clear_mode_state, load_mode_state, save_mode_state, validate_and_cleanup_persisted_state, ModeState};
use super::routing_pbr;
use crate::conf;
use crate::helpers::shell_cmd;
use crate::WG_QUICKRS_CONFIG_FILE;
use ipnet::Ipv4Net;
use std::collections::HashSet;
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SystemMode {
    Host,
    Router,
}

impl From<&str> for SystemMode {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "router" => SystemMode::Router,
            _ => SystemMode::Host,
        }
    }
}

impl From<SystemMode> for String {
    fn from(mode: SystemMode) -> Self {
        match mode {
            SystemMode::Host => "host".to_string(),
            SystemMode::Router => "router".to_string(),
        }
    }
}

#[derive(Error, Debug)]
pub enum ModeError {
    #[error("Cannot switch mode: peers are configured")]
    PeersExist,
    #[error("Invalid LAN CIDR: {0}")]
    InvalidCidr(String),
    #[error("Routing error: {0}")]
    RoutingError(String),
    #[error("Persistence error: {0}")]
    PersistenceError(String),
    #[error("Config error: {0}")]
    ConfigError(String),
}

// Validate CIDR format using ipnet
fn validate_cidr(cidr: &str) -> Result<(), ModeError> {
    Ipv4Net::from_str(cidr)
        .map_err(|e| ModeError::InvalidCidr(format!("Invalid CIDR format '{}': {}", cidr, e)))?;
    Ok(())
}

// Switch between Host and Router Mode
pub fn switch_mode(
    target_mode: SystemMode,
    lan_cidr: Option<String>,
) -> Result<(), ModeError> {
    // Load config once at the beginning (fixes duplicate loading issue)
    let mut config = conf::util::get_config()
        .map_err(|e| ModeError::ConfigError(format!("Failed to load config: {}", e)))?;
    
    // Get current mode
    let current_mode = SystemMode::from(config.agent.router.mode.as_str());
    
    // If already in the target mode and just updating LAN CIDR, allow it
    if current_mode == target_mode && target_mode == SystemMode::Router {
        // Just updating LAN CIDR, not switching modes - allow even with peers
        if let Some(new_cidr) = &lan_cidr {
            return update_lan_cidr(new_cidr);
        }
        // No change needed
        return Ok(());
    }
    
    // STEP 3: Check if peers exist before allowing mode switch
    // Only allow switch if no peers exist (or only the agent's own peer exists)
    let peer_count = config.network.peers.len();
    // If there are peers other than the agent itself, block the switch
    if peer_count > 1 {
        return Err(ModeError::PeersExist);
    }
    
    match target_mode {
        SystemMode::Router => {
            // Switching to Router Mode
            // LAN CIDR is required
            let cidr = lan_cidr
                .ok_or_else(|| ModeError::InvalidCidr("LAN CIDR is required for Router Mode".to_string()))?;
            
            // Validate CIDR format using proper validation
            validate_cidr(&cidr)?;
            
            // Step 1: Enable packet forwarding
            if let Err(e) = enable_packet_forwarding() {
                return Err(ModeError::RoutingError(format!("Failed to enable packet forwarding: {}", e)));
            }
            
            // Step 2: Enable firewall rules (NAT/MASQUERADE and forwarding)
            // Firewall will get LAN CIDR from config if not provided
            if let Err(e) = crate::firewall::enable_router_mode_firewall(&cidr) {
                log::warn!("Failed to enable firewall rules: {} (continuing anyway)", e);
                // Don't fail the mode switch, but log the warning
            }
            
            // Update config with LAN CIDR if it wasn't already set
            if config.agent.router.lan_cidr.is_none() {
                config.agent.router.lan_cidr = Some(cidr.clone());
                let _ = conf::util::set_config(&mut config);
            }
            
            // Step 3: Persist mode state
            let state = ModeState {
                last_mode: SystemMode::Router,
                lan_cidr: Some(cidr.clone()),
                peer_table_ids: std::collections::HashMap::new(),
                prefix_active_backup: std::collections::HashMap::new(),
                peer_first_handshake: std::collections::HashMap::new(),
                peer_last_online_state: std::collections::HashMap::new(),
                peer_last_successful_ping: std::collections::HashMap::new(),
                peer_lan_access: std::collections::HashMap::new(),
                auto_failover: false,
                primary_exit_node: None,
            };
            
            if let Err(e) = save_mode_state(&state) {
                // Rollback: disable forwarding
                let _ = disable_packet_forwarding();
                return Err(ModeError::PersistenceError(format!("Failed to save mode state: {}", e)));
            }
            
            // Step 4: Update config file (final step - no rollback needed if this fails, state is already persisted)
            update_config_mode(SystemMode::Router, Some(&cidr))
                .map_err(|e| ModeError::ConfigError(format!("Failed to update config: {}", e)))?;
            
            // Step 7: Create routing tables and PBR rules for existing peers (STEP 4)
            // This ensures all existing peers have routing tables when switching to Router Mode
            let config = conf::util::get_config()
                .map_err(|e| ModeError::ConfigError(format!("Failed to load config: {}", e)))?;
            let wg_interface = &config.network.name;
            let lan_interface = routing_pbr::find_lan_interface()
                .unwrap_or_else(|_| "eth0".to_string()); // Fallback to eth0
            
            let mut peers_with_default = Vec::new();
            
            for (peer_id, _peer) in &config.network.peers {
                // Skip the agent's own peer
                if *peer_id == config.network.this_peer {
                    continue;
                }
                
                // Create routing table for this peer
                match routing_pbr::create_peer_routing_table(peer_id) {
                    Ok(table_id) => {
                        // Get peer's advertised routes
                        let routes = routing_pbr::get_peer_advertised_routes(peer_id, &config.network);
                        
                        // Install routes into peer's table
                        if let Err(e) = routing_pbr::install_peer_routes(peer_id, table_id, &routes, wg_interface) {
                            log::warn!("Failed to install routes for existing peer {} when switching to Router Mode: {}", peer_id, e);
                        } else {
                            log::info!("Created routing table {} for existing peer {} with {} routes", table_id, peer_id, routes.len());
                        }
                        
                        // Install PBR rules for this peer
                        if let Err(e) = routing_pbr::install_pbr_rules_for_peer(peer_id, table_id, &routes, &lan_interface) {
                            log::warn!("Failed to install PBR rules for peer {}: {}", peer_id, e);
                        } else {
                            log::info!("Successfully installed PBR rules for peer {}", peer_id);
                        }
                        
                        // Track peers with default route for exit node selection
                        if routes.contains(&"0.0.0.0/0".to_string()) || routes.contains(&"default".to_string()) {
                            peers_with_default.push(*peer_id);
                        }
                    }
                    Err(e) => {
                        log::warn!("Failed to create routing table for existing peer {} when switching to Router Mode: {}", peer_id, e);
                    }
                }
            }
            
            // Set first peer with default route as exit node if none is set
            if let Some(first_peer) = peers_with_default.first() {
                if routing_pbr::get_exit_node().unwrap_or(None).is_none() {
                    log::info!("Setting first peer with default route as exit node: {}", first_peer);
                    if let Err(e) = routing_pbr::set_exit_node(first_peer, Some(&config.network)) {
                        log::warn!("Failed to set exit node: {}", e);
                    }
                }
            }
        }
        SystemMode::Host => {
            // Switching to Host Mode
            // Load persisted state if it exists
            let state = load_mode_state()
                .map_err(|e| ModeError::PersistenceError(format!("Failed to load mode state: {}", e)))?;
            
            // Step 1: Disable firewall rules and remove PBR rules
            if let Err(e) = crate::firewall::disable_router_mode_firewall() {
                log::warn!("Failed to disable firewall rules: {} (continuing anyway)", e);
            }
            
            // Remove all PBR rules
            let config = conf::util::get_config()
                .map_err(|e| ModeError::ConfigError(format!("Failed to load config: {}", e)))?;
            
            for (peer_id, _) in &config.network.peers {
                // Skip the host peer
                if *peer_id == config.network.this_peer {
                    continue;
                }
                
                if let Ok(Some(table_id)) = super::routing_pbr::get_peer_table_id(peer_id) {
                    if let Err(e) = super::routing_pbr::remove_pbr_rules_for_peer(peer_id, table_id) {
                        log::warn!("Failed to remove PBR rules for peer {}: {} (continuing anyway)", peer_id, e);
                    }
                }
            }
            
            // Step 2: Disable packet forwarding
            if let Err(e) = disable_packet_forwarding() {
                return Err(ModeError::RoutingError(format!("Failed to disable packet forwarding: {}", e)));
            }
            
            // Step 3: Delete all peer-specific routing tables
            if let Some(ref state) = state {
                if !state.peer_table_ids.is_empty() {
                    log::info!("Deleting {} peer-specific routing tables...", state.peer_table_ids.len());
                    for (peer_id_str, table_id) in &state.peer_table_ids {
                        // Parse peer_id from string
                        if let Ok(peer_id) = uuid::Uuid::parse_str(peer_id_str) {
                            if let Err(e) = super::routing_pbr::remove_peer_routing_table(&peer_id, *table_id) {
                                log::warn!("Failed to remove routing table {} for peer {}: {}", table_id, peer_id_str, e);
                                // Continue with other tables even if one fails
                            } else {
                                log::info!("Removed routing table {} for peer {}", table_id, peer_id_str);
                            }
                        }
                    }
                }
            }
            
            // Step 4: Clear persisted state
            if let Err(e) = clear_mode_state() {
                // Rollback: re-enable forwarding
                let _ = enable_packet_forwarding();
                return Err(ModeError::PersistenceError(format!("Failed to clear mode state: {}", e)));
            }
            
            // Step 5: Update config file (final step)
            update_config_mode(SystemMode::Host, None)
                .map_err(|e| ModeError::ConfigError(format!("Failed to update config: {}", e)))?;
        }
    }
    
    Ok(())
}

/// Update LAN CIDR without switching modes (for when already in Router Mode)
/// This allows updating the LAN CIDR even when peers are configured
pub fn update_lan_cidr(new_cidr: &str) -> Result<(), ModeError> {
    log::info!("Updating LAN CIDR to: {}", new_cidr);
    
    // Validate each CIDR in the comma-separated list
    for cidr in new_cidr.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
        validate_cidr(cidr)?;
    }
    
    // Load current state
    let mut state = load_mode_state()
        .map_err(|e| ModeError::PersistenceError(format!("Failed to load mode state: {}", e)))?
        .ok_or_else(|| ModeError::PersistenceError("No mode state found".to_string()))?;
    
    let old_cidr = state.lan_cidr.clone();
    
    // Update state with new CIDR
    state.lan_cidr = Some(new_cidr.to_string());
    
    // Save updated state
    save_mode_state(&state)
        .map_err(|e| ModeError::PersistenceError(format!("Failed to save mode state: {}", e)))?;
    
    // Update config file
    update_config_mode(SystemMode::Router, Some(new_cidr))?;
    
    // Remove old firewall rules (if old CIDR existed)
    if let Some(ref old) = old_cidr {
        // We don't need to explicitly remove old rules, enable_router_mode_firewall will handle it
        log::debug!("Previous LAN CIDR was: {}", old);
    }
    
    // Apply new firewall rules
    if let Err(e) = crate::firewall::enable_router_mode_firewall(new_cidr) {
        log::warn!("Failed to update firewall rules: {} (continuing anyway)", e);
    }
    
    // Re-apply routing rules with new CIDR
    // Get the current exit node and re-apply its routes
    if let Some(prefix_state) = state.prefix_active_backup.get("0.0.0.0/0") {
        let exit_node_id = prefix_state.active_peer_id.clone();
        if let Ok(exit_uuid) = uuid::Uuid::parse_str(&exit_node_id) {
            log::info!("Re-applying routes for exit node {} with new LAN CIDR", exit_node_id);
            // Get network config
            if let Ok(config) = conf::util::get_config() {
                if let Err(e) = routing_pbr::set_exit_node(&exit_uuid, Some(&config.network)) {
                    log::warn!("Failed to re-apply exit node routes: {}", e);
                }
            }
        }
    }
    
    log::info!("LAN CIDR updated successfully to: {}", new_cidr);
    Ok(())
}

// Enable packet forwarding
fn enable_packet_forwarding() -> Result<(), ModeError> {
    shell_cmd(&["sysctl", "-w", "net.ipv4.ip_forward=1"])
        .map_err(|e| ModeError::RoutingError(format!("Failed to enable packet forwarding: {}", e)))?;
    Ok(())
}

// Disable packet forwarding
fn disable_packet_forwarding() -> Result<(), ModeError> {
    shell_cmd(&["sysctl", "-w", "net.ipv4.ip_forward=0"])
        .map_err(|e| ModeError::RoutingError(format!("Failed to disable packet forwarding: {}", e)))?;
    Ok(())
}

// Update config file with new mode and LAN CIDR
fn update_config_mode(mode: SystemMode, lan_cidr: Option<&str>) -> Result<(), ModeError> {
    let mut config = conf::util::get_config()
        .map_err(|e| ModeError::ConfigError(format!("Failed to load config: {}", e)))?;
    
    config.agent.router.mode = String::from(mode);
    config.agent.router.lan_cidr = lan_cidr.map(|s| s.to_string());
    
    // Use set_config which updates both file and in-memory config
    conf::util::set_config(&mut config)
        .map_err(|e| ModeError::ConfigError(format!("Failed to update config: {}", e)))?;
    
    Ok(())
}

// Get current system mode from config
pub fn get_current_mode() -> Result<SystemMode, ModeError> {
    let config = conf::util::get_config()
        .map_err(|e| ModeError::ConfigError(format!("Failed to load config: {}", e)))?;
    
    Ok(SystemMode::from(config.agent.router.mode.as_str()))
}

// Check if mode can be switched (no peers should exist)
pub fn can_switch_mode() -> Result<bool, ModeError> {
    let config = conf::util::get_config()
        .map_err(|e| ModeError::ConfigError(format!("Failed to load config: {}", e)))?;
    
    // Check if any peers exist (excluding the agent's own peer)
    let peer_count = config.network.peers.len();
    // If only the agent's own peer exists, we can switch
    // If there are other peers, we cannot switch
    // Note: peer_count includes the agent's own peer (this_peer)
    // So if peer_count == 1, only the agent exists, which is allowed
    // If peer_count > 1, there are additional peers, which blocks the switch
    Ok(peer_count <= 1)
}


// Initialize mode on startup - restore Router Mode if it was active
// This function ensures Router Mode persists across VM/container restarts
pub fn initialize_mode_on_startup() -> Result<(), ModeError> {
    log::info!("Checking for Router Mode restoration on startup...");
    
    // Load persisted state first (this is the source of truth)
    let mut state = match load_mode_state()
        .map_err(|e| ModeError::PersistenceError(format!("Failed to load mode state: {}", e)))?
    {
        Some(s) => {
            log::info!("Loaded persisted state: last_mode={:?}, lan_cidr={:?}", s.last_mode, s.lan_cidr);
            s
        },
        None => {
            // No persisted state - system was in Host Mode or never initialized
            log::info!("No persisted Router Mode state found. Starting in Host Mode.");
            return Ok(());
        }
    };
    
    // Check if config.yml exists
    let config_file_path = WG_QUICKRS_CONFIG_FILE
        .get()
        .ok_or_else(|| ModeError::ConfigError("Config file path not initialized".to_string()))?;
    
    if !config_file_path.exists() {
        // Config file doesn't exist - skip restoration but keep state file intact
        // (user might restore config later)
        log::info!("Config file {:?} does not exist. Skipping Router Mode restoration. State file will be preserved.", config_file_path);
        return Ok(());
    }
    
    // Config exists - validate that persisted peer IDs match current config
    let config = match conf::util::get_config() {
        Ok(c) => c,
        Err(e) => {
            // If config can't be loaded, log warning but don't fail
            // (might be a temporary issue)
            log::warn!("Failed to load config for validation: {}. Skipping state validation.", e);
            // Proceed with restoration anyway (legacy behavior)
            // Restore based on last known mode from persisted state
            return restore_mode_from_state(&state);
        }
    };
    
    // Collect current peer IDs from config (excluding agent's own peer)
    let current_peer_ids: HashSet<String> = config.network.peers
        .keys()
        .filter(|peer_id| *peer_id != &config.network.this_peer)
        .map(|peer_id| peer_id.to_string())
        .collect();
    
    // Validate persisted state against current config
    let is_valid_state = validate_and_cleanup_persisted_state(&mut state, &current_peer_ids);
    
    if !is_valid_state {
        // No matching peers - this is a fresh start with a new config
        log::info!("No matching peers found between persisted state and current config. Clearing persisted state (fresh start detected).");
        if let Err(e) = clear_mode_state() {
            log::warn!("Failed to clear persisted state: {}", e);
        }
        return Ok(());
    }
    
    // State is valid - save cleaned up state and proceed with restoration
    if let Err(e) = save_mode_state(&state) {
        log::warn!("Failed to save cleaned up state: {} (continuing anyway)", e);
    }
    
    // Restore based on last known mode from persisted state
    match state.last_mode {
        SystemMode::Router => {
            // Router Mode was active - we need to restore it
            log::info!("Router Mode detected in persisted state. Restoring Router Mode configuration...");
            
            // Get LAN CIDR from persisted state (this is the user-defined value)
            let lan_cidr = state.lan_cidr.clone()
                .ok_or_else(|| ModeError::InvalidCidr("LAN CIDR is required for Router Mode".to_string()))?;
            
            // Step 1: Enable packet forwarding
            enable_packet_forwarding()
                .map_err(|e| ModeError::RoutingError(format!("Failed to enable packet forwarding: {}", e)))?;
            log::info!("Enabled packet forwarding");
            
            // Step 2: Enable firewall rules
            if let Err(e) = crate::firewall::enable_router_mode_firewall(&lan_cidr) {
                log::warn!("Failed to enable firewall rules: {} (continuing anyway)", e);
            }
            
            // Step 3: Update config file to match persisted state (in case it was out of sync)
            update_config_mode(SystemMode::Router, Some(&lan_cidr))
                .map_err(|e| ModeError::ConfigError(format!("Failed to update config: {}", e)))?;
            
            // Note: Peer route restoration is deferred until after WireGuard interface is created
            // This is handled by restore_peer_routes_after_interface_up()
            
            log::info!("Router Mode base configuration restored on startup. LAN CIDR: {}. Peer routes will be restored after WireGuard interface is up.", lan_cidr);
        }
        SystemMode::Host => {
            // Host Mode was active - check if we're already in Host Mode
            let current_mode = get_current_mode().unwrap_or(SystemMode::Host);
            if current_mode == SystemMode::Host {
                log::info!("System already in Host Mode. No restoration needed.");
                return Ok(());
            }
            
            // Host Mode was active but system is not in Host Mode
            log::info!("Host Mode detected in persisted state. Restoring Host Mode configuration...");
            
            // Disable packet forwarding
            disable_packet_forwarding()
                .map_err(|e| ModeError::RoutingError(format!("Failed to disable packet forwarding: {}", e)))?;
            log::info!("Disabled packet forwarding");
            
            // Update config file to match persisted state
            update_config_mode(SystemMode::Host, None)
                .map_err(|e| ModeError::ConfigError(format!("Failed to update config: {}", e)))?;
            
            log::info!("Host Mode successfully restored on startup");
        }
    }
    
    Ok(())
}

/// Restore mode from persisted state (helper function for error cases)
fn restore_mode_from_state(state: &ModeState) -> Result<(), ModeError> {
    match state.last_mode {
        SystemMode::Router => {
            log::info!("Router Mode detected in persisted state. Restoring Router Mode configuration...");
            
            let lan_cidr = state.lan_cidr.clone()
                .ok_or_else(|| ModeError::InvalidCidr("LAN CIDR is required for Router Mode".to_string()))?;
            
            enable_packet_forwarding()
                .map_err(|e| ModeError::RoutingError(format!("Failed to enable packet forwarding: {}", e)))?;
            log::info!("Enabled packet forwarding");
            
            if let Err(e) = crate::firewall::enable_router_mode_firewall(&lan_cidr) {
                log::warn!("Failed to enable firewall rules: {} (continuing anyway)", e);
            }
            
            update_config_mode(SystemMode::Router, Some(&lan_cidr))
                .map_err(|e| ModeError::ConfigError(format!("Failed to update config: {}", e)))?;
            
            log::info!("Router Mode base configuration restored on startup. LAN CIDR: {}. Peer routes will be restored after WireGuard interface is up.", lan_cidr);
        }
        SystemMode::Host => {
            let current_mode = get_current_mode().unwrap_or(SystemMode::Host);
            if current_mode == SystemMode::Host {
                log::info!("System already in Host Mode. No restoration needed.");
                return Ok(());
            }
            
            log::info!("Host Mode detected in persisted state. Restoring Host Mode configuration...");
            
            disable_packet_forwarding()
                .map_err(|e| ModeError::RoutingError(format!("Failed to disable packet forwarding: {}", e)))?;
            log::info!("Disabled packet forwarding");
            
            update_config_mode(SystemMode::Host, None)
                .map_err(|e| ModeError::ConfigError(format!("Failed to update config: {}", e)))?;
            
            log::info!("Host Mode successfully restored on startup");
        }
    }
    
    Ok(())
}

/// Restore peer routing tables after WireGuard interface is created
/// This should be called after the WireGuard interface is up and running
pub fn restore_peer_routes_after_interface_up() -> Result<(), ModeError> {
    log::info!("Restoring peer routing tables after interface is up...");
    
    // Load persisted state to get peer table IDs
    let mut state = match load_mode_state()
        .map_err(|e| ModeError::PersistenceError(format!("Failed to load mode state: {}", e)))?
    {
        Some(s) => s,
        None => {
            log::debug!("No persisted state found. Skipping peer route restoration.");
            return Ok(());
        }
    };
    
    // Only restore if we were in Router Mode
    if state.last_mode != SystemMode::Router {
        log::debug!("Not in Router Mode. Skipping peer route restoration.");
        return Ok(());
    }
    
    // Get config
    let config = conf::util::get_config()
        .map_err(|e| ModeError::ConfigError(format!("Failed to load config: {}", e)))?;
    
    // Final validation and cleanup pass (in case config changed between startup and interface up)
    let current_peer_ids: HashSet<String> = config.network.peers
        .keys()
        .filter(|peer_id| *peer_id != &config.network.this_peer)
        .map(|peer_id| peer_id.to_string())
        .collect();
    
    let is_valid_state = validate_and_cleanup_persisted_state(&mut state, &current_peer_ids);
    if !is_valid_state {
        log::warn!("Persisted state is no longer valid after interface up. Clearing state.");
        if let Err(e) = clear_mode_state() {
            log::warn!("Failed to clear persisted state: {}", e);
        }
        return Ok(());
    }
    
    let wg_interface = &config.network.name;
    
    // Verify interface exists before trying to install routes
    let interface_check = shell_cmd(&["ip", "link", "show", wg_interface]);
    if interface_check.is_err() {
        log::warn!("WireGuard interface {} does not exist yet. Will retry later.", wg_interface);
        return Err(ModeError::RoutingError(format!("Interface {} does not exist", wg_interface)));
    }
    
    log::info!("WireGuard interface {} exists. Restoring peer routes...", wg_interface);
    
    // Restore routing tables for existing peers
    let mut restored_count = 0;
    let mut failed_count = 0;
    
    for (peer_id, _peer) in &config.network.peers {
        // Skip the agent's own peer
        if *peer_id == config.network.this_peer {
            continue;
        }
        
        // Check if table ID exists in persisted state
        let table_id = if let Some(&existing_table_id) = state.peer_table_ids.get(&peer_id.to_string()) {
            existing_table_id
        } else {
            // Table ID not found in persisted state, create a new one
            match routing_pbr::create_peer_routing_table(peer_id) {
                Ok(id) => {
                    log::info!("Created new routing table {} for peer {}", id, peer_id);
                    // Update local state to match what create_peer_routing_table saved
                    state.peer_table_ids.insert(peer_id.to_string(), id);
                    id
                },
                Err(e) => {
                    log::warn!("Failed to create routing table for peer {}: {}", peer_id, e);
                    failed_count += 1;
                    continue;
                }
            }
        };
        
        // Get peer's advertised routes
        let routes = routing_pbr::get_peer_advertised_routes(peer_id, &config.network);
        
        // Install routes into peer's table
        if let Err(e) = routing_pbr::install_peer_routes(peer_id, table_id, &routes, wg_interface) {
            log::warn!("Failed to restore routes for peer {}: {}", peer_id, e);
            failed_count += 1;
        } else {
            log::info!("Restored routing table {} for peer {} with {} routes", table_id, peer_id, routes.len());
            restored_count += 1;
            
            // Restore PBR rules for this peer
            let lan_interface = routing_pbr::find_lan_interface()
                .unwrap_or_else(|_| "eth0".to_string());
            if let Err(e) = routing_pbr::install_pbr_rules_for_peer(peer_id, table_id, &routes, &lan_interface) {
                log::warn!("Failed to restore PBR rules for peer {}: {}", peer_id, e);
            } else {
                log::info!("Restored PBR rules for peer {}", peer_id);
            }
        }
    }
    
    if restored_count > 0 {
        log::info!("Successfully restored routing tables for {} peer(s)", restored_count);
    }
    if failed_count > 0 {
        log::warn!("Failed to restore routing tables for {} peer(s)", failed_count);
    }
    
    // Restore exit node from persisted state
    if let Some(exit_node_id) = routing_pbr::get_exit_node().unwrap_or(None) {
        // Verify exit node still exists in config before restoring
        let exit_node_id_str = exit_node_id.to_string();
        if current_peer_ids.contains(&exit_node_id_str) {
            log::info!("Restoring exit node: {}", exit_node_id);
            // Clone network to avoid lifetime issues
            let network_clone = config.network.clone();
            if let Err(e) = routing_pbr::set_exit_node(&exit_node_id, Some(&network_clone)) {
                log::warn!("Failed to restore exit node: {}", e);
            } else {
                log::info!("Successfully restored exit node: {}", exit_node_id);
            }
        } else {
            log::warn!("Exit node {} from persisted state no longer exists in config. Skipping exit node restoration.", exit_node_id_str);
        }
    }
    
    // Save cleaned up state after restoration
    if let Err(e) = save_mode_state(&state) {
        log::warn!("Failed to save cleaned up state after route restoration: {}", e);
    }
    
    Ok(())
}

