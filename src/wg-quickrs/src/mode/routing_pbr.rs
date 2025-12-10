// Per-peer routing tables, ip rule priority, Active/Backup selection,
// overlapping prefix logic, default route arbitration
//
// Responsibilities:
// - STEP 4: Create peer-specific routing tables
// - STEP 7: Policy-based routing for overlapping destinations

use crate::helpers::{shell_cmd, parse_lan_cidrs};
use super::persist::{load_mode_state, save_mode_state};
use super::mode::SystemMode;
use thiserror::Error;
use uuid::Uuid;
use wg_quickrs_lib::types::network::Network;
use wg_quickrs_lib::helpers::wg_public_key_from_private_key;
use std::str::FromStr;
use once_cell::sync::Lazy;
use std::sync::{Arc, Mutex, RwLock};
use std::collections::HashMap;
use std::collections::VecDeque;
use std::time::Duration;
use tokio::time::{interval, sleep};

#[derive(Error, Debug)]
pub enum PolicyRoutingError {
    #[error("Table ID error: {0}")]
    TableIdError(String),
    #[error("IP rule error: {0}")]
    IpRuleError(String),
    #[error("Route installation error: {0}")]
    RouteInstallationError(String),
    #[error("Persistence error: {0}")]
    PersistenceError(String),
}

// Cached LAN interface (lazy initialization)
static LAN_INTERFACE_CACHE: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));

// Cached health status for exit nodes (updated by background monitor)
static EXIT_NODE_HEALTH_CACHE: Lazy<Arc<RwLock<HashMap<Uuid, ExitNodeHealth>>>> = 
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

// Health monitoring interval (ping every 1 second, like OPNsense dpinger)
// Peer is marked offline after CONSECUTIVE_FAILURES_THRESHOLD (3) consecutive failed pings
const HEALTH_MONITOR_INTERVAL_SECS: u64 = 1;

// Track ping history for loss and jitter calculation (60 samples = 60 seconds like OPNsense)
const PING_HISTORY_SIZE: usize = 60;

// Ping history entry
#[derive(Debug, Clone)]
struct PingResult {
    #[allow(dead_code)]
    timestamp: u64, // Stored for potential future use (e.g., time-based filtering)
    latency_ms: Option<u64>, // None = packet lost
}

// Per-peer ping history (for loss and jitter calculation)
static PING_HISTORY: Lazy<Arc<RwLock<HashMap<Uuid, VecDeque<PingResult>>>>> = 
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

// In-memory "Up Since" tracking (resets on restart, not persisted)
// Tracks when peer came online in current session (after restart or offline→online transition)
static SESSION_UP_SINCE: Lazy<Arc<RwLock<HashMap<Uuid, u64>>>> = 
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

// Consecutive ping failures per peer (for offline detection)
// Peer is marked offline only after CONSECUTIVE_FAILURES_THRESHOLD failures
static CONSECUTIVE_FAILURES: Lazy<Arc<RwLock<HashMap<Uuid, u32>>>> = 
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

// Number of consecutive ping failures required to mark peer as offline
const CONSECUTIVE_FAILURES_THRESHOLD: u32 = 3;

// Fail-back delay: seconds the primary must be online before switching back
const FAILBACK_STABILITY_SECS: u64 = 60;

// Track when primary exit node came back online (for fail-back timing)
// Key: peer_id, Value: timestamp when peer came back online
static PRIMARY_ONLINE_SINCE: Lazy<Arc<RwLock<Option<(Uuid, u64)>>>> = 
    Lazy::new(|| Arc::new(RwLock::new(None)));

// Calculate packet loss and jitter from ping history (like OPNsense dpinger)
fn calculate_loss_and_jitter(history: &VecDeque<PingResult>) -> (Option<f64>, Option<u64>) {
    if history.is_empty() {
        return (None, None);
    }
    
    // Calculate packet loss: (failed pings / total pings) * 100
    let total_pings = history.len();
    let failed_pings = history.iter().filter(|r| r.latency_ms.is_none()).count();
    let packet_loss_percent = if total_pings > 0 {
        Some((failed_pings as f64 / total_pings as f64) * 100.0)
    } else {
        None
    };
    
    // Calculate jitter: standard deviation of successful ping latencies
    let successful_latencies: Vec<u64> = history
        .iter()
        .filter_map(|r| r.latency_ms)
        .collect();
    
    let jitter_ms = if successful_latencies.len() >= 2 {
        // Calculate mean
        let mean = successful_latencies.iter().sum::<u64>() as f64 / successful_latencies.len() as f64;
        
        // Calculate variance
        let variance = successful_latencies
            .iter()
            .map(|&x| {
                let diff = x as f64 - mean;
                diff * diff
            })
            .sum::<f64>() / successful_latencies.len() as f64;
        
        // Standard deviation = jitter
        let std_dev = variance.sqrt();
        Some(std_dev.round() as u64)
    } else {
        None
    };
    
    (packet_loss_percent, jitter_ms)
}

// Parsed IP rule structure for efficient rule management
#[derive(Debug, Clone)]
struct ParsedRule {
    priority: u32,
    table_id: Option<u32>,
    from: Option<String>,
    to: Option<String>,
    iif: Option<String>,
}

// Parse ip rule show output into structured rules
fn parse_ip_rules(output: &str) -> Vec<ParsedRule> {
    let mut rules = Vec::new();
    
    for line in output.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }
        
        // Extract priority (first part, ends with ':')
        let priority_str = parts[0].trim_end_matches(':');
        let priority = match priority_str.parse::<u32>() {
            Ok(p) => p,
            Err(_) => continue,
        };
        
        let mut rule = ParsedRule {
            priority,
            table_id: None,
            from: None,
            to: None,
            iif: None,
        };
        
        // Parse rule components
        let mut i = 1;
        while i < parts.len() {
            match parts[i] {
                "from" => {
                    if i + 1 < parts.len() {
                        rule.from = Some(parts[i + 1].to_string());
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "to" => {
                    if i + 1 < parts.len() {
                        rule.to = Some(parts[i + 1].to_string());
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "iif" => {
                    if i + 1 < parts.len() {
                        rule.iif = Some(parts[i + 1].to_string());
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "lookup" => {
                    if i + 1 < parts.len() {
                        if let Ok(table_id) = parts[i + 1].parse::<u32>() {
                            rule.table_id = Some(table_id);
                        }
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                _ => i += 1,
            }
        }
        
        rules.push(rule);
    }
    
    rules
}

// Get and cache ip rules (parse once, reuse)
fn get_ip_rules_cached() -> Result<Vec<ParsedRule>, PolicyRoutingError> {
    let output = shell_cmd(&["ip", "rule", "show"])
        .map_err(|e| PolicyRoutingError::IpRuleError(format!("Failed to get ip rules: {}", e)))?;
    let output_str = String::from_utf8_lossy(&output.stdout);
    Ok(parse_ip_rules(&output_str))
}


// Create routing table for a peer
// Assigns a unique table ID in range 1000-9999 and persists it
pub fn create_peer_routing_table(peer_id: &Uuid) -> Result<u32, PolicyRoutingError> {
    let peer_id_str = peer_id.to_string();
    log::debug!("[create_peer_routing_table] Starting for peer {}", peer_id_str);
    
    // Load current state to check existing table IDs
    log::debug!("[create_peer_routing_table] Loading mode state...");
    let mut state = match load_mode_state() {
        Ok(Some(s)) => {
            log::debug!("[create_peer_routing_table] Loaded mode state successfully");
            s
        }
        Ok(None) => {
            log::error!("[create_peer_routing_table] No mode state found");
            return Err(PolicyRoutingError::PersistenceError("No mode state found".to_string()));
        }
        Err(e) => {
            log::error!("[create_peer_routing_table] Failed to load mode state: {}", e);
            return Err(PolicyRoutingError::PersistenceError(format!("Failed to load mode state: {}", e)));
        }
    };
    
    // Check if table ID already exists for this peer
    if let Some(&existing_table_id) = state.peer_table_ids.get(&peer_id_str) {
        log::info!("[create_peer_routing_table] Peer {} already has routing table {}", peer_id_str, existing_table_id);
        return Ok(existing_table_id);
    }
    
    // Find next available table ID in range 1000-9999
    log::debug!("[create_peer_routing_table] Finding available table ID...");
    let mut table_id = 1000u32;
    let used_table_ids: std::collections::HashSet<u32> = state.peer_table_ids.values().cloned().collect();
    log::debug!("[create_peer_routing_table] Currently used table IDs: {:?}", used_table_ids);
    
    while table_id <= 9999 {
        if !used_table_ids.contains(&table_id) {
            break;
        }
        table_id += 1;
    }
    
    if table_id > 9999 {
        log::error!("[create_peer_routing_table] No available table IDs in range 1000-9999");
        return Err(PolicyRoutingError::TableIdError(
            "No available table IDs in range 1000-9999".to_string()
        ));
    }
    
    log::info!("[create_peer_routing_table] Selected table ID {} for peer {}", table_id, peer_id_str);
    
    // Store the mapping
    state.peer_table_ids.insert(peer_id_str.clone(), table_id);
    log::debug!("[create_peer_routing_table] Saving mode state with new table mapping...");
    match save_mode_state(&state) {
        Ok(_) => {
            log::debug!("[create_peer_routing_table] Mode state saved successfully");
        }
        Err(e) => {
            log::error!("[create_peer_routing_table] Failed to save mode state: {}", e);
            return Err(PolicyRoutingError::PersistenceError(format!("Failed to save mode state: {}", e)));
        }
    }
    
    // Linux creates routing tables on first use, so we just need to ensure it exists
    // by adding a dummy route and removing it, or we can just add routes directly
    // For now, we'll create it implicitly when we add the first route
    
    log::info!("[create_peer_routing_table] Successfully created routing table {} for peer {}", table_id, peer_id_str);
    Ok(table_id)
}

// Validate a route CIDR string
// Returns true if the route is valid and should be installed
fn validate_route(route: &str) -> bool {
    // Check if it's a default route
    if route == "0.0.0.0/0" || route == "default" {
        return true;
    }
    
    // Try to parse as IPv4 CIDR
    if let Ok(net) = ipnet::Ipv4Net::from_str(route) {
        // Check if the network address matches the CIDR (e.g., 10.100.105.1/24 is invalid, should be 10.100.105.0/24)
        let network_addr = net.network();
        let addr_parts: Vec<&str> = route.split('/').collect();
        if addr_parts.len() == 2 {
            if let Ok(prefix_len) = addr_parts[1].parse::<u8>() {
                if prefix_len <= 32 {
                    // Check if the address is actually the network address
                    // For /24, the last octet should be 0
                    // For /32, any address is valid
                    if prefix_len == 32 {
                        return true; // /32 is always valid
                    }
                    // For other prefix lengths, verify it's a valid network address
                    let expected_network = ipnet::Ipv4Net::new(network_addr, prefix_len).unwrap();
                    return expected_network.to_string() == route;
                }
            }
        }
    }
    
    false
}

// Install peer's advertised routes into peer's table
// Routes are installed into the peer-specific table, not the main table
// Only valid routes that the peer is configured to advertise are installed
pub fn install_peer_routes(
    peer_id: &Uuid,
    table_id: u32,
    routes: &[String],
    wg_interface: &str,
) -> Result<(), PolicyRoutingError> {
    let peer_id_str = peer_id.to_string();
    
    if routes.is_empty() {
        log::debug!("No routes to install for peer {}", peer_id_str);
        return Ok(());
    }
    
    // Filter and validate routes - only install valid routes
    let valid_routes: Vec<&String> = routes.iter()
        .filter(|route| {
            if validate_route(route) {
                true
            } else {
                log::warn!("Skipping invalid route {} for peer {} (not a valid CIDR or network address)", route, peer_id_str);
                false
            }
        })
        .collect();
    
    let valid_count = valid_routes.len();
    let invalid_count = routes.len() - valid_count;
    
    if valid_routes.is_empty() {
        log::debug!("No valid routes to install for peer {} (filtered {} invalid routes)", peer_id_str, invalid_count);
        return Ok(());
    }
    
    log::info!("Installing {} valid routes into table {} for peer {} (filtered {} invalid routes)", 
        valid_count, table_id, peer_id_str, invalid_count);
    
    // Pre-allocate table_id string to avoid repeated allocations
    let table_id_str = table_id.to_string();
    
    for route in &valid_routes {
        // Handle default route specially
        let route_str = if *route == "default" { "0.0.0.0/0" } else { route };
        
        // Install route into peer's table: ip route add <cidr> dev <interface> table <table_id>
        let cmd = &[
            "ip", "route", "add",
            route_str,
            "dev", wg_interface,
            "table", &table_id_str,
        ];
        
        match shell_cmd(cmd) {
            Ok(_) => {
                log::debug!("Installed route {} into table {} for peer {}", route_str, table_id, peer_id_str);
            }
            Err(e) => {
                // If route already exists, try replace
                let error_str = e.to_string();
                if error_str.contains("File exists") || error_str.contains("RTNETLINK answers: File exists") {
                    log::debug!("Route {} already exists in table {}, replacing...", route_str, table_id);
                    let replace_cmd = &[
                        "ip", "route", "replace",
                        route_str,
                        "dev", wg_interface,
                        "table", &table_id_str,
                    ];
                    if let Err(replace_err) = shell_cmd(replace_cmd) {
                        return Err(PolicyRoutingError::RouteInstallationError(
                            format!("Failed to replace route {} in table {}: {}", route_str, table_id, replace_err)
                        ));
                    }
                } else {
                    log::warn!("Failed to install route {} into table {} for peer {}: {} (skipping)", route_str, table_id, peer_id_str, e);
                    // Continue with other routes instead of failing completely
                }
            }
        }
    }
    
    log::info!("Successfully installed {} routes into table {} for peer {}", valid_count, table_id, peer_id_str);
    Ok(())
}

// Install PBR rules for a peer's routes
// Routes traffic from LAN interface to peer's routing table based on destination
pub fn install_pbr_rules_for_peer(
    peer_id: &Uuid,
    table_id: u32,
    routes: &[String],
    lan_interface: &str,
) -> Result<(), PolicyRoutingError> {
    let peer_id_str = peer_id.to_string();
    
    if routes.is_empty() {
        log::debug!("No routes to install PBR rules for peer {}", peer_id_str);
        return Ok(());
    }
    
    // Filter out default routes - those are handled by set_exit_node() for the exit node only
    let specific_routes: Vec<&String> = routes.iter()
        .filter(|r| *r != "0.0.0.0/0" && *r != "default")
        .collect();
    
    let specific_routes_count = specific_routes.len();
    
    if specific_routes_count == 0 {
        log::debug!("No specific routes to install PBR rules for peer {} (only default route(s))", peer_id_str);
        return Ok(());
    }
    
    log::info!("Installing PBR rules for {} specific routes from {} to table {} (peer {})", 
        specific_routes_count, lan_interface, table_id, peer_id_str);
    
    // Pre-allocate strings to avoid repeated allocations
    let table_id_str = table_id.to_string();
    let base_priority = 10000 + (table_id % 1000);
    
    for route in specific_routes {
        // Specific routes: higher priority (10000+), checked first
        let priority = base_priority;
        let priority_str = priority.to_string();
        
        // Install ip rule: iif <lan_interface> to <route> lookup <table_id>
        // Use "iif" (input interface) instead of "from" (source IP)
        let cmd = &[
            "ip", "rule", "add",
            "iif", lan_interface,
            "to", route,
            "lookup", &table_id_str,
            "priority", &priority_str,
        ];
        
        match shell_cmd(cmd) {
            Ok(_) => {
                log::debug!("Installed PBR rule: from {} to {} -> table {} (priority {})", 
                    lan_interface, route, table_id, priority);
            }
            Err(e) => {
                let error_str = e.to_string();
                // If rule already exists, try to replace it
                if error_str.contains("File exists") || error_str.contains("RTNETLINK answers: File exists") {
                    log::debug!("PBR rule already exists, replacing...");
                    // Delete old rule first
                    let _ = shell_cmd(&[
                        "ip", "rule", "del",
                        "iif", lan_interface,
                        "to", route,
                        "lookup", &table_id_str,
                    ]);
                    // Add new rule
                    if let Err(replace_err) = shell_cmd(cmd) {
                        return Err(PolicyRoutingError::IpRuleError(
                            format!("Failed to replace PBR rule for {} -> {}: {}", route, table_id, replace_err)
                        ));
                    }
                } else {
                    return Err(PolicyRoutingError::IpRuleError(
                        format!("Failed to install PBR rule for {} -> {}: {}", route, table_id, e)
                    ));
                }
            }
        }
    }
    
    log::info!("Successfully installed {} PBR rules for peer {} (skipped default route - handled by exit node)", 
        specific_routes_count, peer_id_str);
    Ok(())
}

// Remove PBR rules for a peer (optimized with parsed rules)
pub fn remove_pbr_rules_for_peer(
    peer_id: &Uuid,
    table_id: u32,
) -> Result<(), PolicyRoutingError> {
    let peer_id_str = peer_id.to_string();
    
    log::info!("Removing PBR rules for table {} (peer {})", table_id, peer_id_str);
    
    // Get parsed rules (cached)
    let rules = get_ip_rules_cached()?;
    let mut removed_count = 0;
    let _priority_str = table_id.to_string();
    
    for rule in &rules {
        // Check if this rule references our table and is not an exit node rule
        if rule.table_id == Some(table_id) && rule.priority < 20000 {
            // Rule exists (we just parsed it), delete it
            let priority_str = rule.priority.to_string();
            let del_cmd = &["ip", "rule", "del", "priority", &priority_str];
            if let Err(e) = shell_cmd(del_cmd) {
                log::warn!("Failed to delete PBR rule with priority {}: {}", rule.priority, e);
            } else {
                log::debug!("Deleted PBR rule with priority {} for table {}", rule.priority, table_id);
                removed_count += 1;
            }
        }
    }
    
    if removed_count > 0 {
        log::info!("Removed {} PBR rules for peer {}", removed_count, peer_id_str);
    } else {
        log::debug!("No PBR rules found for table {}", table_id);
    }
    
    Ok(())
}

// Update PBR rules for a peer (remove old, install new)
pub fn update_pbr_rules_for_peer(
    peer_id: &Uuid,
    network: &Network,
    lan_interface: &str,
) -> Result<(), PolicyRoutingError> {
    // Get the table ID for this peer
    let table_id = match get_peer_table_id(peer_id)? {
        Some(id) => id,
        None => {
            log::debug!("No routing table found for peer {}, skipping PBR rule update", peer_id);
            return Ok(());
        }
    };
    
    // Remove old PBR rules
    remove_pbr_rules_for_peer(peer_id, table_id)?;
    
    // Get current routes
    let routes = get_peer_advertised_routes(peer_id, network);
    
    // Install new PBR rules
    install_pbr_rules_for_peer(peer_id, table_id, &routes, lan_interface)?;
    
    log::info!("Updated PBR rules for peer {} ({} routes)", peer_id, routes.len());
    Ok(())
}

// Set exit node for default route
// network: Optional network config to avoid deadlock (if None, will load config)
pub fn set_exit_node(peer_id: &Uuid, network: Option<&Network>) -> Result<(), PolicyRoutingError> {
    // Get network config - use provided network or load config (avoid deadlock)
    if let Some(net) = network {
        set_exit_node_impl(peer_id, net)
    } else {
        // Fallback: load config if not provided (should be avoided when called from respond.rs)
        let config = crate::conf::util::get_config()
            .map_err(|e| PolicyRoutingError::PersistenceError(format!("Failed to load config: {}", e)))?;
        set_exit_node_impl(peer_id, &config.network)
    }
}

// Internal implementation that does the actual work
fn set_exit_node_impl(peer_id: &Uuid, network: &Network) -> Result<(), PolicyRoutingError> {
    let peer_id_str = peer_id.to_string();
    
    // Load current state ONCE at the beginning - reuse throughout the function
    let mut state = load_mode_state()
        .map_err(|e| PolicyRoutingError::PersistenceError(format!("Failed to load mode state: {}", e)))?
        .ok_or_else(|| PolicyRoutingError::PersistenceError("No mode state found".to_string()))?;
    
    // Get table ID for this peer - use loaded state to avoid redundant load
    let table_id = match state.peer_table_ids.get(&peer_id_str).copied() {
        Some(id) => id,
        None => {
            return Err(PolicyRoutingError::TableIdError(
                format!("No routing table found for peer {}", peer_id_str)
            ));
        }
    };
    
    // Get current exit node if any
    let old_exit_node = state.prefix_active_backup
        .get("0.0.0.0/0")
        .and_then(|ps| state.peer_table_ids.get(&ps.active_peer_id))
        .copied();
    
    // Remove old exit node rule if different
    let wg_interface = &network.name;
    let wg_subnet = network.subnet.to_string();
    
    // Parse rules once for all cleanup operations
    let all_rules = get_ip_rules_cached()?;
    
    if let Some(old_table_id) = old_exit_node {
        if old_table_id != table_id {
            log::info!("Removing old exit node rule for table {}", old_table_id);
            // Find and remove old exit node rules using parsed rules
            for rule in &all_rules {
                // Remove LAN exit node rules
                if rule.to == Some("0.0.0.0/0".to_string()) 
                    && rule.table_id == Some(old_table_id) 
                    && rule.priority >= 20000 {
                    let priority_str = rule.priority.to_string();
                    let _ = shell_cmd(&["ip", "rule", "del", "priority", &priority_str]);
                    log::debug!("Removed old LAN exit node rule with priority {}", rule.priority);
                }
                // Remove WireGuard peer exit node rules
                if rule.from == Some(wg_subnet.clone())
                    && rule.iif == Some(wg_interface.to_string())
                    && rule.to == Some("0.0.0.0/0".to_string())
                    && rule.table_id == Some(old_table_id)
                    && rule.priority >= 20000 {
                    let priority_str = rule.priority.to_string();
                    let _ = shell_cmd(&["ip", "rule", "del", "priority", &priority_str]);
                    log::debug!("Removed old WireGuard peer exit node rule with priority {}", rule.priority);
                }
            }
            // Also remove old LAN exception rules if they exist
            if let Some(lan_cidr_str) = &state.lan_cidr {
                let lan_cidrs = parse_lan_cidrs(lan_cidr_str);
                let lan_interface = find_lan_interface().ok();
                for lan_cidr in &lan_cidrs {
                    if let Some(ref iface) = lan_interface {
                        // Remove eth0 LAN exception rule
                        let _ = shell_cmd(&[
                            "ip", "rule", "del",
                            "iif", iface,
                            "to", lan_cidr,
                            "lookup", "main",
                        ]);
                    }
                    // Remove old subnet-wide WireGuard peer LAN exception rule (migration)
                    let _ = shell_cmd(&[
                        "ip", "rule", "del",
                        "from", &wg_subnet,
                        "iif", wg_interface,
                        "to", lan_cidr,
                        "lookup", "main",
                    ]);
                    // Remove per-peer LAN exception rules
                    for (peer_id, peer) in &network.peers {
                        if *peer_id == network.this_peer {
                            continue;
                        }
                        let peer_addr = format!("{}/32", peer.address);
                        let _ = shell_cmd(&[
                            "ip", "rule", "del",
                            "from", &peer_addr,
                            "iif", wg_interface,
                            "to", lan_cidr,
                            "lookup", "main",
                        ]);
                    }
                }
            }
        }
    }
    
    // Dynamically manage 0.0.0.0/0 in WireGuard allowed IPs
    // Remove 0.0.0.0/0 from old exit node if different
    // IMPORTANT: Do this BEFORE updating the state, so we can get the old exit node
    let old_exit_node_peer_id_str_opt = state.prefix_active_backup
        .get("0.0.0.0/0")
        .map(|ps| ps.active_peer_id.clone());
    
    log::debug!("[set_exit_node_impl] Current exit node in state: {:?}, new exit node: {}", 
                old_exit_node_peer_id_str_opt, peer_id_str);
    
    if let Some(old_exit_node_peer_id_str) = &old_exit_node_peer_id_str_opt {
        log::debug!("[set_exit_node_impl] Found old exit node: {}, comparing with new: {}", old_exit_node_peer_id_str, peer_id_str);
        if old_exit_node_peer_id_str != &peer_id_str {
            log::debug!("[set_exit_node_impl] Old and new exit nodes are different, removing 0.0.0.0/0 from old exit node");
            if let Ok(old_peer_uuid) = Uuid::parse_str(&old_exit_node_peer_id_str) {
                if let Some(old_peer) = network.peers.get(&old_peer_uuid) {
                    let old_public_key = wg_public_key_from_private_key(&old_peer.private_key);
                    let old_public_key_b64 = old_public_key.to_base64();
                    log::info!("Removing 0.0.0.0/0 from old exit node {} (public key: {})", old_exit_node_peer_id_str, old_public_key_b64);
                    
                    // When removing 0.0.0.0/0 from old exit node, always use the peer's own address
                    // This ensures the peer can still be pinged/reached after losing exit node status
                    let peer_addr = format!("{}/32", old_peer.address);
                    let mut current_allowed_ips = vec![peer_addr.clone()];
                    log::debug!("Setting allowed IPs for old exit node {} to peer's own address: {}", old_exit_node_peer_id_str, peer_addr);
                    
                    // Also preserve any other non-default routes from the connection (excluding 0.0.0.0/0 and the router's address)
                    for (conn_id, conn_details) in &network.connections {
                        if conn_id.contains(&old_peer_uuid) && conn_id.contains(&network.this_peer) {
                            let (other_id, allowed_ips) = if conn_id.a == old_peer_uuid {
                                (&conn_id.b, &conn_details.allowed_ips_a_to_b)
                            } else {
                                (&conn_id.a, &conn_details.allowed_ips_b_to_a)
                            };
                            if other_id == &network.this_peer {
                                // This is the connection to the router
                                // Get router's address from the network's this_peer
                                let router_addr = network.peers.get(&network.this_peer)
                                    .map(|p| format!("{}/32", p.address))
                                    .unwrap_or_else(|| "10.100.105.1/32".to_string());
                                for ip in allowed_ips {
                                    let ip_str = ip.to_string();
                                    // Exclude 0.0.0.0/0, default, router's address, and peer's own address (already added)
                                    if ip_str != "0.0.0.0/0" 
                                        && ip_str != "default" 
                                        && ip_str != router_addr
                                        && ip_str != peer_addr {
                                        current_allowed_ips.push(ip_str);
                                    }
                                }
                                break;
                            }
                        }
                    }
                    
                    // Remove 0.0.0.0/0 and set remaining allowed IPs
                    let allowed_ips_str = current_allowed_ips.join(",");
                    log::info!("Setting allowed IPs for old exit node {} to: {}", old_exit_node_peer_id_str, allowed_ips_str);
                    if let Err(e) = shell_cmd(&["wg", "set", wg_interface, "peer", &old_public_key_b64, 
                                                "allowed-ips", &allowed_ips_str]) {
                        log::warn!("Failed to remove 0.0.0.0/0 from old exit node {}: {}", old_exit_node_peer_id_str, e);
                    } else {
                        log::info!("Removed 0.0.0.0/0 from old exit node {} and set allowed IPs to: {}", old_exit_node_peer_id_str, allowed_ips_str);
                    }
                }
            }
        }
    }
    
    // Update state
    let mut backup_peer_ids = Vec::new();
    
    // Find all peers with default route - cache routes to avoid redundant computation
    // Since we already computed routes for backup peers check, reuse that logic
    // Optimize: Use get_peers_with_default_route which is already optimized
    let peers_with_default = get_peers_with_default_route(network);
    for other_peer_id in &peers_with_default {
        if *other_peer_id != *peer_id {
            backup_peer_ids.push(other_peer_id.to_string());
        }
    }
    
    state.prefix_active_backup.insert(
        "0.0.0.0/0".to_string(),
        super::persist::PrefixState {
            active_peer_id: peer_id_str.clone(),
            backup_peer_ids,
        },
    );
    
    save_mode_state(&state)
        .map_err(|e| PolicyRoutingError::PersistenceError(format!("Failed to save mode state: {}", e)))?;
    
    // Install exit node rule: iif <lan_interface> to 0.0.0.0/0 lookup <table_id> priority 20000
    // But first, add exception rule for LAN CIDR to keep local traffic local
    // Find LAN interface (cached)
    let lan_interface = find_lan_interface()?;
    let priority = 20000 + (table_id % 1000);
    let priority_str = priority.to_string();
    let table_id_str = table_id.to_string();
    
    // Get LAN CIDRs from state to create exception rules (supports multiple comma-separated CIDRs)
    if let Some(lan_cidr_str) = &state.lan_cidr {
        let lan_cidrs = parse_lan_cidrs(lan_cidr_str);
        
        // Install exception rule: LAN traffic stays in main table (priority < 20000)
        let exception_priority = priority - 1; // One less than default route rule
        
        // Add per-peer LAN exception rules for WireGuard peers
        // This allows individual control over which peers can reach the local LAN
        let wg_peer_lan_base_priority = exception_priority - 100; // Start 100 below eth0 exception
        
        for (cidr_idx, lan_cidr) in lan_cidrs.iter().enumerate() {
            // Remove existing exception rule if any
            let _ = shell_cmd(&[
                "ip", "rule", "del",
                "iif", &lan_interface,
                "to", lan_cidr,
                "lookup", "main",
            ]);
            
            // Add exception rule: iif <lan_interface> to <lan_cidr> lookup main
            // Use slightly different priorities for each CIDR to avoid conflicts
            let cidr_exception_priority = exception_priority - (cidr_idx as u32);
            let cidr_exception_priority_str = cidr_exception_priority.to_string();
            
            let exception_cmd = &[
                "ip", "rule", "add",
                "iif", &lan_interface,
                "to", lan_cidr,
                "lookup", "main",
                "priority", &cidr_exception_priority_str,
            ];
            
            if let Err(e) = shell_cmd(exception_cmd) {
                log::warn!("Failed to install LAN exception rule for {}: {} (continuing anyway)", lan_cidr, e);
            } else {
                log::info!("Installed LAN exception rule: {} -> main table (priority {})", lan_cidr, cidr_exception_priority);
            }
            
            // First, remove any existing per-peer LAN rules (clean slate)
            // Remove old subnet-wide rule if it exists (migration from old format)
            let _ = shell_cmd(&[
                "ip", "rule", "del",
                "from", &wg_subnet,
                "iif", wg_interface,
                "to", lan_cidr,
                "lookup", "main",
            ]);
            
            // Remove existing per-peer rules (priority range 19800-19899)
            for rule in &all_rules {
                if rule.iif == Some(wg_interface.to_string())
                    && rule.to == Some(lan_cidr.to_string())
                    && rule.table_id.is_none() // lookup main doesn't have a numeric table_id in our parsing
                    && rule.priority >= wg_peer_lan_base_priority as u32
                    && rule.priority < exception_priority as u32 {
                    let priority_str = rule.priority.to_string();
                    let _ = shell_cmd(&["ip", "rule", "del", "priority", &priority_str]);
                    log::debug!("Removed old per-peer LAN rule with priority {}", rule.priority);
                }
            }
            
            // Also try to remove by matching criteria for each peer
            for (peer_id, peer) in &network.peers {
                if *peer_id == network.this_peer {
                    continue; // Skip the router itself
                }
                let peer_addr = format!("{}/32", peer.address);
                let _ = shell_cmd(&[
                    "ip", "rule", "del",
                    "from", &peer_addr,
                    "iif", wg_interface,
                    "to", lan_cidr,
                    "lookup", "main",
                ]);
            }
            
            // Add rules for each peer that has LAN access
            let mut peer_index = 0u32;
            for (peer_id, peer) in &network.peers {
                if *peer_id == network.this_peer {
                    continue; // Skip the router itself
                }
                
                let peer_id_str = peer_id.to_string();
                
                // Check if peer has LAN access (default is true if not specified)
                let has_lan_access = state.peer_lan_access
                    .get(&peer_id_str)
                    .copied()
                    .unwrap_or(true); // Default to true (has LAN access)
                
                if has_lan_access {
                    let peer_addr = format!("{}/32", peer.address);
                    // Use unique priority: base + (cidr_index * 100) + peer_index
                    let peer_priority = wg_peer_lan_base_priority + (cidr_idx as u32 * 100) + peer_index;
                    let peer_priority_str = peer_priority.to_string();
                    
                    let peer_lan_cmd = &[
                        "ip", "rule", "add",
                        "from", &peer_addr,
                        "iif", wg_interface,
                        "to", lan_cidr,
                        "lookup", "main",
                        "priority", &peer_priority_str,
                    ];
                    
                    if let Err(e) = shell_cmd(peer_lan_cmd) {
                        log::warn!("Failed to install LAN access rule for peer {} ({}) to {}: {}", peer.name, peer_addr, lan_cidr, e);
                    } else {
                        log::info!("Installed LAN access rule for peer {} ({}) to {}: -> main table (priority {})", 
                            peer.name, peer_addr, lan_cidr, peer_priority);
                    }
                } else if cidr_idx == 0 {
                    // Only log once per peer, not for each CIDR
                    log::info!("Peer {} ({}) does not have LAN access - no rule installed", peer.name, peer.address);
                }
                
                peer_index += 1;
            }
        } // end for lan_cidr
    }
    
    // Install default route rule: iif <lan_interface> to 0.0.0.0/0 lookup <table_id>
    // First, remove any existing rule for this table using parsed rules
    for rule in &all_rules {
        // Check if this rule matches our criteria (iif lan_interface and lookup our table)
        if rule.iif == Some(lan_interface.clone())
            && rule.to == Some("0.0.0.0/0".to_string())
            && rule.table_id == Some(table_id)
            && rule.priority >= 20000 {
            let priority_str = rule.priority.to_string();
            let _ = shell_cmd(&["ip", "rule", "del", "priority", &priority_str]);
            log::debug!("Removed old exit node rule with priority {}", rule.priority);
        }
    }
    
    // Also try to remove by matching criteria in case priority-based removal failed
    let _ = shell_cmd(&[
        "ip", "rule", "del",
        "iif", &lan_interface,
        "to", "0.0.0.0/0",
        "lookup", &table_id_str,
    ]);
    
    let cmd = &[
        "ip", "rule", "add",
        "iif", &lan_interface,
        "to", "0.0.0.0/0",
        "lookup", &table_id_str,
        "priority", &priority_str,
    ];
    
    if let Err(e) = shell_cmd(cmd) {
        let error_str = e.to_string();
        // If rule already exists, try to replace it
        if error_str.contains("File exists") || error_str.contains("RTNETLINK answers: File exists") {
            log::debug!("Exit node rule already exists, replacing...");
            // Delete by matching criteria
            let _ = shell_cmd(&[
                "ip", "rule", "del",
                "iif", &lan_interface,
                "to", "0.0.0.0/0",
                "lookup", &table_id_str,
            ]);
            // Try adding again
            if let Err(e2) = shell_cmd(cmd) {
                return Err(PolicyRoutingError::IpRuleError(
                    format!("Failed to install exit node rule after replacement attempt: {}", e2)
                ));
            } else {
                log::info!("Successfully replaced exit node rule for table {}", table_id);
            }
        } else {
            return Err(PolicyRoutingError::IpRuleError(
                format!("Failed to install exit node rule: {}", e)
            ));
        }
    }
    
    // Install PBR rule for WireGuard peers: from <wg_subnet> iif <wg_interface> to 0.0.0.0/0 lookup <table_id>
    // This allows all WireGuard peers to use the exit node for internet traffic
    // (wg_interface and wg_subnet are already defined above)
    
    // Remove old WireGuard peer rules for this table if they exist (using already parsed rules)
    for rule in &all_rules {
        // Check if this rule matches our criteria (from wg_subnet, iif wg_interface, lookup our table)
        if rule.from == Some(wg_subnet.clone())
            && rule.iif == Some(wg_interface.to_string())
            && rule.to == Some("0.0.0.0/0".to_string())
            && rule.table_id == Some(table_id)
            && rule.priority >= 20000 {
            let priority_str = rule.priority.to_string();
            let _ = shell_cmd(&["ip", "rule", "del", "priority", &priority_str]);
            log::debug!("Removed old WireGuard peer exit node rule with priority {}", rule.priority);
        }
    }
    
    // Add PBR rule for WireGuard peers: from <wg_subnet> iif <wg_interface> to 0.0.0.0/0 lookup <table_id>
    // Use priority 20001 (one higher than LAN rule) so it takes precedence
    let wg_peer_priority = priority + 1;
    let wg_peer_priority_str = wg_peer_priority.to_string();
    let wg_peer_cmd = &[
        "ip", "rule", "add",
        "from", &wg_subnet,
        "iif", wg_interface,
        "to", "0.0.0.0/0",
        "lookup", &table_id_str,
        "priority", &wg_peer_priority_str,
    ];
    
    if let Err(e) = shell_cmd(wg_peer_cmd) {
        log::warn!("Failed to install WireGuard peer exit node rule: {} (continuing anyway)", e);
    } else {
        log::info!("Installed WireGuard peer exit node rule: from {} iif {} to 0.0.0.0/0 -> table {} (priority {})", 
            wg_subnet, wg_interface, table_id, wg_peer_priority);
    }
    
    // Install default route in the peer's table
    
    // Install default route: ip route add 0.0.0.0/0 dev <wg_interface> table <table_id>
    let route_cmd = &[
        "ip", "route", "add",
        "0.0.0.0/0",
        "dev", wg_interface,
        "table", &table_id_str,
    ];
    
    // Try to add, if it exists, replace it
    if let Err(e) = shell_cmd(route_cmd) {
        let error_str = e.to_string();
        if error_str.contains("File exists") || error_str.contains("RTNETLINK answers: File exists") {
            log::debug!("Default route already exists in table {}, replacing...", table_id);
            let replace_cmd = &[
                "ip", "route", "replace",
                "0.0.0.0/0",
                "dev", wg_interface,
                "table", &table_id_str,
            ];
            if let Err(replace_err) = shell_cmd(replace_cmd) {
                return Err(PolicyRoutingError::RouteInstallationError(
                    format!("Failed to install default route in table {}: {}", table_id, replace_err)
                ));
            }
        } else {
            return Err(PolicyRoutingError::RouteInstallationError(
                format!("Failed to install default route in table {}: {}", table_id, e)
            ));
        }
    }
    
    // Add 0.0.0.0/0 to new exit node
    let new_peer = network.peers.get(peer_id)
        .ok_or_else(|| PolicyRoutingError::TableIdError(format!("Peer {} not found in network", peer_id_str)))?;
    let new_public_key = wg_public_key_from_private_key(&new_peer.private_key);
    let new_public_key_b64 = new_public_key.to_base64();
    
    // Get current allowed IPs for the new peer (excluding 0.0.0.0/0)
    let mut current_allowed_ips = Vec::new();
    for (conn_id, conn_details) in &network.connections {
        if conn_id.contains(peer_id) && conn_id.contains(&network.this_peer) {
            let (other_id, allowed_ips) = if conn_id.a == *peer_id {
                (&conn_id.b, &conn_details.allowed_ips_a_to_b)
            } else {
                (&conn_id.a, &conn_details.allowed_ips_b_to_a)
            };
            if other_id == &network.this_peer {
                // This is the connection to the router
                for ip in allowed_ips {
                    let ip_str = ip.to_string();
                    if ip_str != "0.0.0.0/0" && ip_str != "default" {
                        current_allowed_ips.push(ip_str);
                    }
                }
                break;
            }
        }
    }
    
    // If no other IPs, use the peer's own address
    if current_allowed_ips.is_empty() {
        current_allowed_ips.push(format!("{}/32", new_peer.address));
    }
    
    // Add 0.0.0.0/0 to the list
    current_allowed_ips.push("0.0.0.0/0".to_string());
    let allowed_ips_str = current_allowed_ips.join(",");
    
    log::info!("Adding 0.0.0.0/0 to new exit node {} (public key: {})", peer_id_str, new_public_key_b64);
    if let Err(e) = shell_cmd(&["wg", "set", wg_interface, "peer", &new_public_key_b64, 
                                "allowed-ips", &allowed_ips_str]) {
        log::warn!("Failed to add 0.0.0.0/0 to new exit node {}: {}", peer_id_str, e);
        // Don't fail the entire operation, but log the warning
    } else {
        log::info!("Successfully added 0.0.0.0/0 to exit node {}", peer_id_str);
    }
    
    Ok(())
}

// Get current exit node
pub fn get_exit_node() -> Result<Option<Uuid>, PolicyRoutingError> {
    let state = match load_mode_state()
        .map_err(|e| PolicyRoutingError::PersistenceError(format!("Failed to load mode state: {}", e)))?
    {
        Some(s) => s,
        None => return Ok(None),
    };
    
    if let Some(prefix_state) = state.prefix_active_backup.get("0.0.0.0/0") {
        if let Ok(peer_id) = Uuid::parse_str(&prefix_state.active_peer_id) {
            return Ok(Some(peer_id));
        }
    }
    
    Ok(None)
}

// Get Smart Gateway (auto-failover) status
pub fn get_auto_failover() -> Result<bool, PolicyRoutingError> {
    let state = match load_mode_state()
        .map_err(|e| PolicyRoutingError::PersistenceError(format!("Failed to load mode state: {}", e)))?
    {
        Some(s) => s,
        None => return Ok(false), // Default to disabled
    };
    
    Ok(state.auto_failover)
}

// Set Smart Gateway (auto-failover) status
pub fn set_auto_failover(enabled: bool) -> Result<(), PolicyRoutingError> {
    let mut state = match load_mode_state()
        .map_err(|e| PolicyRoutingError::PersistenceError(format!("Failed to load mode state: {}", e)))?
    {
        Some(s) => s,
        None => return Err(PolicyRoutingError::PersistenceError("No mode state found - enable Router Mode first".to_string())),
    };
    
    state.auto_failover = enabled;
    
    save_mode_state(&state)
        .map_err(|e| PolicyRoutingError::PersistenceError(format!("Failed to save mode state: {}", e)))?;
    
    log::info!("Smart Gateway (auto-failover) set to: {}", enabled);
    Ok(())
}

// Get primary exit node (user's preferred gateway for fail-back)
pub fn get_primary_exit_node() -> Result<Option<Uuid>, PolicyRoutingError> {
    let state = match load_mode_state()
        .map_err(|e| PolicyRoutingError::PersistenceError(format!("Failed to load mode state: {}", e)))?
    {
        Some(s) => s,
        None => return Ok(None),
    };
    
    if let Some(primary_str) = state.primary_exit_node {
        if let Ok(peer_id) = Uuid::parse_str(&primary_str) {
            return Ok(Some(peer_id));
        }
    }
    
    Ok(None)
}

// Set primary exit node (user's preferred gateway for fail-back)
pub fn set_primary_exit_node(peer_id: Option<Uuid>) -> Result<(), PolicyRoutingError> {
    let mut state = match load_mode_state()
        .map_err(|e| PolicyRoutingError::PersistenceError(format!("Failed to load mode state: {}", e)))?
    {
        Some(s) => s,
        None => return Err(PolicyRoutingError::PersistenceError("No mode state found - enable Router Mode first".to_string())),
    };
    
    state.primary_exit_node = peer_id.map(|id| id.to_string());
    
    save_mode_state(&state)
        .map_err(|e| PolicyRoutingError::PersistenceError(format!("Failed to save mode state: {}", e)))?;
    
    if let Some(id) = peer_id {
        log::debug!("Smart Gateway: Set primary exit node to {}", id);
    } else {
        log::debug!("Smart Gateway: Cleared primary exit node");
    }
    Ok(())
}

// Get all peers that advertise default route
// Optimized: Cache routes per peer to avoid redundant computation
pub fn get_peers_with_default_route(network: &Network) -> Vec<Uuid> {
    let mut peers = Vec::new();
    // Cache routes per peer to avoid recomputing for the same peer
    let mut route_cache: std::collections::HashMap<Uuid, Vec<String>> = std::collections::HashMap::new();
    
    for (peer_id, _) in &network.peers {
        // Skip this router itself - it can't be an exit node
        if *peer_id == network.this_peer {
            continue;
        }
        
        // Check cache first
        let routes = route_cache.entry(*peer_id).or_insert_with(|| {
            get_peer_advertised_routes(peer_id, network)
        });
        
        if routes.contains(&"0.0.0.0/0".to_string()) || routes.contains(&"default".to_string()) {
            peers.push(*peer_id);
        }
    }
    
    peers
}

// Health status for an exit node
#[derive(Debug, Clone)]
pub struct ExitNodeHealth {
    pub peer_id: Uuid,
    pub is_online: bool,
    pub last_handshake: Option<u64>, // Unix timestamp in seconds
    pub first_handshake: Option<u64>, // Unix timestamp in seconds (when peer first came online via ping - used for "Up Since")
    pub latency_ms: Option<u64>,     // Latency in milliseconds (current/average)
    pub packet_loss_percent: Option<f64>, // Packet loss percentage (0.0-100.0)
    pub jitter_ms: Option<u64>,      // Jitter in milliseconds (latency variation)
    pub transfer_rx: u64,           // Bytes received
    pub transfer_tx: u64,           // Bytes sent
    pub endpoint: Option<String>,   // Endpoint address:port
}

// Get health status for exit nodes (reads from cache, updated by background monitor)
pub fn get_exit_node_health(network: &Network, _wg_interface: &str) -> Vec<ExitNodeHealth> {
    let peers_with_default = get_peers_with_default_route(network);
    let cache = EXIT_NODE_HEALTH_CACHE.read().unwrap();
    
    // Return cached health status for peers with default routes
    peers_with_default
        .iter()
        .filter_map(|peer_id| cache.get(peer_id).cloned())
        .collect()
}

// Background health monitoring task (runs continuously, updates cache)
// Matches OPNsense dpinger behavior: single lightweight ping every 1 second
pub async fn start_health_monitor() -> std::io::Result<()> {
    let mut ticker = interval(Duration::from_secs(HEALTH_MONITOR_INTERVAL_SECS)); // Check every 1 second
    
    loop {
        ticker.tick().await;
        
        // Only monitor if in Router Mode
        if let Ok(Some(state)) = load_mode_state() {
            if state.last_mode == SystemMode::Router {
                if let Ok(config) = crate::conf::util::get_config() {
                    let wg_interface = config.network.name.clone();
                    let peers_with_default = get_peers_with_default_route(&config.network);
                    let network = config.network.clone();
                    
                    // Monitor each peer concurrently (spawn tasks to avoid blocking)
                    for peer_id in peers_with_default {
                        if let Some(peer) = network.peers.get(&peer_id) {
                            let peer_id_clone = peer_id;
                            let peer_clone = peer.clone();
                            let wg_interface_clone = wg_interface.clone();
                            let network_clone = network.clone();
                            let cache = EXIT_NODE_HEALTH_CACHE.clone();
                            
                            // Clone peer name for logging
                            let peer_name = peer.name.clone();
                            
                            // Spawn async task to check health (non-blocking)
                            tokio::spawn(async move {
                                let health = check_peer_health_impl_async(
                                    &network_clone,
                                    peer_id_clone,
                                    &peer_clone,
                                    &wg_interface_clone,
                                ).await;
                                
                                // Check for status transition before updating cache
                                let mut cache = cache.write().unwrap();
                                let old_health = cache.get(&peer_id_clone);
                                
                                // Log status transitions
                                if let Some(old) = old_health {
                                    if old.is_online != health.is_online {
                                        let peer_id_short = &peer_id_clone.to_string()[..8];
                                        if health.is_online {
                                            // Offline → Online
                                            let handshake_info = health.last_handshake
                                                .map(|ts| {
                                                    let now = std::time::SystemTime::now()
                                                        .duration_since(std::time::UNIX_EPOCH)
                                                        .map(|d| d.as_secs())
                                                        .unwrap_or(0);
                                                    let ago = now.saturating_sub(ts);
                                                    format!("handshake {}s ago", ago)
                                                })
                                                .unwrap_or_else(|| "ping successful".to_string());
                                            let latency_info = health.latency_ms
                                                .map(|l| format!(", latency {}ms", l))
                                                .unwrap_or_default();
                                            log::info!(
                                                "Peer {} ({}) status changed: Offline → Online ({}{})",
                                                peer_name, peer_id_short, handshake_info, latency_info
                                            );
                                            
                                            // Smart Gateway fail-back: Track when primary comes back online
                                            if let Ok(true) = get_auto_failover() {
                                                if let Ok(Some(primary_id)) = get_primary_exit_node() {
                                                    if primary_id == peer_id_clone {
                                                        let now = std::time::SystemTime::now()
                                                            .duration_since(std::time::UNIX_EPOCH)
                                                            .map(|d| d.as_secs())
                                                            .unwrap_or(0);
                                                        let mut tracker = PRIMARY_ONLINE_SINCE.write().unwrap();
                                                        *tracker = Some((peer_id_clone, now));
                                                        log::info!(
                                                            "Smart Gateway: Primary {} came back online, will fail-back in {}s if stable",
                                                            peer_name, FAILBACK_STABILITY_SECS
                                                        );
                                                    }
                                                }
                                            }
                                        } else {
                                            // Online → Offline
                                            let handshake_info = old.last_handshake
                                                .map(|ts| {
                                                    let now = std::time::SystemTime::now()
                                                        .duration_since(std::time::UNIX_EPOCH)
                                                        .map(|d| d.as_secs())
                                                        .unwrap_or(0);
                                                    let ago = now.saturating_sub(ts);
                                                    format!("last handshake was {}s ago", ago)
                                                })
                                                .unwrap_or_else(|| "no handshake".to_string());
                                            let loss_info = health.packet_loss_percent
                                                .map(|l| format!(", {:.0}% packet loss", l))
                                                .unwrap_or_default();
                                            log::warn!(
                                                "Peer {} ({}) status changed: Online → Offline ({}{})",
                                                peer_name, peer_id_short, handshake_info, loss_info
                                            );
                                            
                                            // Smart Gateway: Check if this peer is the current exit node and auto-failover is enabled
                                            if let Ok(Some(current_exit)) = get_exit_node() {
                                                if current_exit == peer_id_clone {
                                                    if let Ok(true) = get_auto_failover() {
                                                        log::info!("Smart Gateway: Current exit node {} went offline, triggering failover...", peer_name);
                                                        
                                                        // Find best healthy alternative from cache
                                                        let best_alternative = cache.iter()
                                                            .filter(|(id, h)| **id != peer_id_clone && h.is_online)
                                                            .min_by_key(|(_, h)| h.latency_ms.unwrap_or(u64::MAX))
                                                            .map(|(id, h)| (*id, h.latency_ms));
                                                        
                                                        if let Some((new_exit_id, latency)) = best_alternative {
                                                            // Load config for set_exit_node
                                                            if let Ok(config) = crate::conf::util::get_config() {
                                                                let new_peer_name = config.network.peers.get(&new_exit_id)
                                                                    .map(|p| p.name.clone())
                                                                    .unwrap_or_else(|| new_exit_id.to_string());
                                                                
                                                                // Save current exit as primary before switching (for fail-back)
                                                                if let Err(e) = set_primary_exit_node(Some(peer_id_clone)) {
                                                                    log::warn!("Smart Gateway: Failed to save primary exit node: {}", e);
                                                                }
                                                                
                                                                match set_exit_node(&new_exit_id, Some(&config.network)) {
                                                                    Ok(_) => {
                                                                        let latency_info = latency.map(|l| format!(" ({}ms)", l)).unwrap_or_default();
                                                                        log::info!(
                                                                            "Smart Gateway: Switched from {} to {}{} (will fail-back after {}s)",
                                                                            peer_name, new_peer_name, latency_info, FAILBACK_STABILITY_SECS
                                                                        );
                                                                    }
                                                                    Err(e) => {
                                                                        log::error!("Smart Gateway: Failed to switch to {}: {}", new_peer_name, e);
                                                                    }
                                                                }
                                                            }
                                                        } else {
                                                            log::warn!("Smart Gateway: No healthy alternatives available for failover");
                                                        }
                                                    }
                                                }
                                            }
                                            
                                            // Clear fail-back tracking if primary went offline
                                            let mut primary_tracker = PRIMARY_ONLINE_SINCE.write().unwrap();
                                            if let Some((tracked_id, _)) = *primary_tracker {
                                                if tracked_id == peer_id_clone {
                                                    *primary_tracker = None;
                                                    log::debug!("Smart Gateway: Primary {} went offline, resetting fail-back timer", peer_name);
                                                }
                                            }
                                        }
                                    }
                                }
                                
                                // Update cache
                                cache.insert(peer_id_clone, health.clone());
                                
                                // Smart Gateway fail-back: Check if primary has been online long enough
                                if let Ok(true) = get_auto_failover() {
                                    let tracker = PRIMARY_ONLINE_SINCE.read().unwrap();
                                    if let Some((primary_id, online_since)) = *tracker {
                                        drop(tracker); // Release lock before doing work
                                        
                                        // Only check if this health update is for the primary
                                        if peer_id_clone == primary_id && health.is_online {
                                            let now = std::time::SystemTime::now()
                                                .duration_since(std::time::UNIX_EPOCH)
                                                .map(|d| d.as_secs())
                                                .unwrap_or(0);
                                            let online_duration = now.saturating_sub(online_since);
                                            
                                            // Check if stable for FAILBACK_STABILITY_SECS
                                            if online_duration >= FAILBACK_STABILITY_SECS {
                                                // Check if we're on a different exit node
                                                if let Ok(Some(current_exit)) = get_exit_node() {
                                                    if current_exit != primary_id {
                                                        log::info!(
                                                            "Smart Gateway: Primary {} has been online for {}s, triggering fail-back...",
                                                            peer_name, online_duration
                                                        );
                                                        
                                                        // Load config and switch back
                                                        if let Ok(config) = crate::conf::util::get_config() {
                                                            match set_exit_node(&primary_id, Some(&config.network)) {
                                                                Ok(_) => {
                                                                    log::info!(
                                                                        "Smart Gateway: Switched back to primary {}",
                                                                        peer_name
                                                                    );
                                                                    // Clear primary tracking - we're back on primary
                                                                    let _ = set_primary_exit_node(None);
                                                                    let mut tracker = PRIMARY_ONLINE_SINCE.write().unwrap();
                                                                    *tracker = None;
                                                                }
                                                                Err(e) => {
                                                                    log::error!(
                                                                        "Smart Gateway: Failed to switch back to primary {}: {}",
                                                                        peer_name, e
                                                                    );
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            });
                        }
                    }
                }
            }
        }
        
        // Small delay to prevent tight loop
        sleep(Duration::from_millis(100)).await;
    }
}

// Async version of check_peer_health_impl (non-blocking)
async fn check_peer_health_impl_async(
    _network: &Network,
    peer_id: Uuid,
    peer: &wg_quickrs_lib::types::network::Peer,
    wg_interface: &str,
) -> ExitNodeHealth {
    // Load persisted state (for last_online_state and last_successful_ping only)
    // first_handshake is now session-only (resets on restart)
    let mut mode_state = load_mode_state().unwrap_or(None);
    let mut last_online_state_map = mode_state.as_mut()
        .map(|s| std::mem::take(&mut s.peer_last_online_state))
        .unwrap_or_default();
    let mut last_successful_ping_map = mode_state.as_mut()
        .map(|s| std::mem::take(&mut s.peer_last_successful_ping))
        .unwrap_or_default();
    
    let peer_id_str = peer_id.to_string();
    let public_key = wg_public_key_from_private_key(&peer.private_key);
    let public_key_b64 = public_key.to_base64();
    
    // Get WireGuard dump output (async, non-blocking)
    use tokio::process::Command as TokioCommand;
    let wg_dump = match TokioCommand::new("wg")
        .arg("show")
        .arg(wg_interface)
        .arg("dump")
        .output()
        .await
    {
        Ok(output) if output.status.success() => String::from_utf8_lossy(&output.stdout).to_string(),
        _ => {
            // Return default health if can't get dump
            // Calculate loss/jitter from existing history if available
            let ping_history = PING_HISTORY.clone();
            let history = ping_history.read().unwrap();
            let (packet_loss_percent, jitter_ms) = if let Some(peer_history) = history.get(&peer_id) {
                calculate_loss_and_jitter(peer_history)
            } else {
                (None, None)
            };
            
            return ExitNodeHealth {
                peer_id,
                is_online: false,
                last_handshake: None,
                first_handshake: None,
                latency_ms: None,
                packet_loss_percent,
                jitter_ms,
                transfer_rx: 0,
                transfer_tx: 0,
                endpoint: None,
            };
        }
    };
    
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    // Find this peer in WireGuard dump
    let mut last_handshake: Option<u64> = None;
    let mut transfer_rx: u64 = 0;
    let mut transfer_tx: u64 = 0;
    let mut endpoint: Option<String> = None;
    
    for line in wg_dump.lines().skip(1) {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 8 {
            continue;
        }
        
        if parts[0] == public_key_b64 {
            // Parse endpoint (column 2)
            if parts.len() > 2 && parts[2] != "(none)" && !parts[2].is_empty() && parts[2] != "off" {
                endpoint = Some(parts[2].to_string());
            }
            
            // Parse last handshake (column 4, in seconds since epoch)
            if parts.len() > 4 {
                last_handshake = parts[4].parse::<u64>().ok();
            }
            
            // Parse transfer stats (columns 5 and 6)
            if parts.len() > 6 {
                transfer_rx = parts[5].parse::<u64>().unwrap_or(0);
                transfer_tx = parts[6].parse::<u64>().unwrap_or(0);
            }
            break;
        }
    }
    
    // Check connectivity using ping (non-blocking async version)
    // Ping the peer's tunnel IP (peer.address) via the WireGuard interface
    let (ping_succeeded, latency_ms) = check_peer_connectivity_async(&peer.address.to_string(), wg_interface).await;
    
    // Apply consecutive failures threshold for offline detection
    // Peer is only marked offline after CONSECUTIVE_FAILURES_THRESHOLD consecutive failures
    let consecutive_failures = CONSECUTIVE_FAILURES.clone();
    let is_online = {
        let mut failures = consecutive_failures.write().unwrap();
        let failure_count = failures.entry(peer_id).or_insert(0);
        
        if ping_succeeded {
            // Ping succeeded - reset failure counter, peer is online
            *failure_count = 0;
            true
        } else {
            // Ping failed - increment counter
            *failure_count = failure_count.saturating_add(1);
            // Only mark offline after threshold consecutive failures
            *failure_count < CONSECUTIVE_FAILURES_THRESHOLD
        }
    };
    
    // Track ping history for loss and jitter calculation (like OPNsense dpinger)
    // Note: Uses ping_succeeded (actual ping result) not is_online (threshold-based status)
    let ping_history = PING_HISTORY.clone();
    let mut history = ping_history.write().unwrap();
    let peer_history = history.entry(peer_id).or_insert_with(|| VecDeque::with_capacity(PING_HISTORY_SIZE));
    
    // Add current ping result to history
    peer_history.push_back(PingResult {
        timestamp: now,
        latency_ms, // None if ping_succeeded is false
    });
    
    // Keep only last 60 results (60 second window, like OPNsense)
    while peer_history.len() > PING_HISTORY_SIZE {
        peer_history.pop_front();
    }
    
    // Calculate packet loss and jitter from history (like OPNsense dpinger)
    let (packet_loss_percent, jitter_ms) = calculate_loss_and_jitter(peer_history);
    
    // Release lock before continuing
    drop(history);
    
    // Track "Up Since" based on ping-based online status
    // "Up Since" resets on restart and tracks when peer came online in current session
    let was_online_previously = last_online_state_map.get(&peer_id_str).copied().unwrap_or(false);
    
    // Get session-only "Up Since" tracking (in-memory, resets on restart)
    let session_up_since = SESSION_UP_SINCE.clone();
    let first_handshake = {
        if is_online {
            // Peer is currently online - update last successful ping time
            last_successful_ping_map.insert(peer_id_str.clone(), now);
            
            // Track when peer first came online in this session (after restart or offline→online transition)
            let mut session_up_since_map = session_up_since.write().unwrap();
            if !was_online_previously {
                // Peer transitioned from offline to online - set "Up Since" to now
                session_up_since_map.insert(peer_id, now);
                Some(now)
            } else {
                // Peer was already online - get existing "Up Since" from session, or set to now if not tracked
                session_up_since_map.get(&peer_id).copied().or_else(|| {
                    // First time seeing this peer online in this session (e.g., after restart)
                    session_up_since_map.insert(peer_id, now);
                    Some(now)
                })
            }
        } else {
            // Peer is offline - use last successful ping time (when it was last seen online)
            // This is used for "Down Since" display
            last_successful_ping_map.get(&peer_id_str).copied()
        }
    };
    
    // Update the last known online state
    last_online_state_map.insert(peer_id_str.clone(), is_online);
    
    // Save updated state back to persistence (but NOT first_handshake - it's session-only)
    // IMPORTANT: Reload the state fresh before saving to avoid overwriting concurrent changes (e.g., lan_cidr updates)
    if let Ok(Some(mut fresh_state)) = load_mode_state() {
        // Only update the health-related fields we manage
        fresh_state.peer_last_online_state = last_online_state_map;
        fresh_state.peer_last_successful_ping = last_successful_ping_map;
        let _ = save_mode_state(&fresh_state);
    }
    
    ExitNodeHealth {
        peer_id,
        is_online,
        last_handshake,
        first_handshake,
        latency_ms,
        packet_loss_percent,
        jitter_ms,
        transfer_rx,
        transfer_tx,
        endpoint,
    }
}

// Async version: Check peer connectivity and measure latency using ping (non-blocking)
// Returns (is_online, latency_ms)
// Pings the peer's tunnel IP (peer.address) via the WireGuard interface
// Uses 3 packets with 2 second timeout per packet, total timeout 10 seconds
// Uses WireGuard interface directly to avoid routing table issues when switching gateways
async fn check_peer_connectivity_async(peer_address: &str, wg_interface: &str) -> (bool, Option<u64>) {
    use tokio::process::Command;
    
    // Use async Command to avoid blocking the runtime
    // Format: ping -I <wg_interface> -c 1 -W 1 -w 2 <peer_tunnel_ip>
    // Matches OPNsense dpinger: single lightweight ping with short timeout
    // -I forces ping to use the WireGuard interface directly, bypassing routing table issues
    // peer_address is the peer's tunnel IP (e.g., 10.100.105.2)
    // -c 1: single packet (like dpinger)
    // -W 1: 1 second timeout per packet
    // -w 2: 2 second total timeout (fails fast)
    let output = match Command::new("ping")
        .arg("-I")
        .arg(wg_interface)
        .arg("-c")
        .arg("1")
        .arg("-W")
        .arg("1")
        .arg("-w")
        .arg("2")
        .arg(peer_address)
        .output()
        .await
    {
        Ok(output) if output.status.success() => output,
        _ => return (false, None),
    };
    
    let output_str = String::from_utf8_lossy(&output.stdout);
    
    // Parse ping output for all time=XX.X ms values
    // Example: "64 bytes from 10.100.105.2: icmp_seq=1 ttl=64 time=42.3 ms"
    let mut latencies = Vec::new();
    for line in output_str.lines() {
        if let Some(time_pos) = line.find("time=") {
            let time_part = &line[time_pos + 5..];
            if let Some(ms_pos) = time_part.find(" ms") {
                let time_str = &time_part[..ms_pos];
                if let Ok(time_val) = time_str.parse::<f64>() {
                    latencies.push(time_val);
                }
            }
        }
    }
    
    // If we got at least one successful ping, peer is online
    if !latencies.is_empty() {
        // Calculate average latency
        let sum: f64 = latencies.iter().sum();
        let avg = sum / latencies.len() as f64;
        (true, Some(avg.round() as u64))
    } else {
        // No successful pings, peer is offline
        (false, None)
    }
}



// Helper: Find LAN interface (cached)
pub fn find_lan_interface() -> Result<String, PolicyRoutingError> {
    // Check cache first
    {
        let cache = LAN_INTERFACE_CACHE.lock().unwrap();
        if let Some(ref cached) = *cache {
            return Ok(cached.clone());
        }
    }
    
    // Not cached, detect interface
    let lan_cidr = match load_mode_state()
        .map_err(|e| PolicyRoutingError::PersistenceError(format!("Failed to load mode state: {}", e)))?
    {
        Some(state) => state.lan_cidr,
        None => None,
    };
    
    let interface = if let Some(cidr) = lan_cidr {
        // Use similar logic as firewall.rs
        let parts: Vec<&str> = cidr.split('/').collect();
        if parts.len() == 2 {
            let network = parts[0];
            let network_parts: Vec<&str> = network.split('.').collect();
            if network_parts.len() >= 3 {
                let network_prefix = format!("{}.{}.{}", network_parts[0], network_parts[1], network_parts[2]);
                
                let ip_output = shell_cmd(&["ip", "-4", "addr", "show"])
                    .map_err(|e| PolicyRoutingError::IpRuleError(format!("Failed to list interfaces: {}", e)))?;
                
                let ip_output_str = String::from_utf8_lossy(&ip_output.stdout);
                let mut current_interface: Option<String> = None;
                
                for line in ip_output_str.lines() {
                    if line.contains(':') && !line.starts_with("    ") && !line.starts_with(" ") {
                        let iface_part = line.split(':').nth(1);
                        if let Some(iface) = iface_part {
                            let iface_name = iface.split('@').next().unwrap_or("").trim();
                            if !iface_name.is_empty() && iface_name != "lo" {
                                current_interface = Some(iface_name.to_string());
                            }
                        }
                    } else if let Some(iface) = &current_interface {
                        if line.contains("inet") && line.contains(&network_prefix) {
                            let result = iface.clone();
                            // Cache the result
                            {
                                let mut cache = LAN_INTERFACE_CACHE.lock().unwrap();
                                *cache = Some(result.clone());
                            }
                            return Ok(result);
                        }
                    }
                }
            }
        }
        
        // Fallback to common interface names
        for iface in &["eth0", "ens3", "enp0s3", "enp1s0"] {
            if shell_cmd(&["ip", "addr", "show", iface]).is_ok() {
                let result = iface.to_string();
                // Cache the result
                {
                    let mut cache = LAN_INTERFACE_CACHE.lock().unwrap();
                    *cache = Some(result.clone());
                }
                return Ok(result);
            }
        }
        
        // Default
        "eth0".to_string()
    } else {
        // Fallback to common interface names
        for iface in &["eth0", "ens3", "enp0s3", "enp1s0"] {
            if shell_cmd(&["ip", "addr", "show", iface]).is_ok() {
                let result = iface.to_string();
                // Cache the result
                {
                    let mut cache = LAN_INTERFACE_CACHE.lock().unwrap();
                    *cache = Some(result.clone());
                }
                return Ok(result);
            }
        }
        "eth0".to_string()
    };
    
    // Cache the result
    {
        let mut cache = LAN_INTERFACE_CACHE.lock().unwrap();
        *cache = Some(interface.clone());
    }
    
    Ok(interface)
}

// Set active peer for overlapping prefix (for future use with other prefixes)
pub fn set_active_peer_for_prefix(
    prefix: &str,
    active_peer_id: &str,
    _backup_peer_ids: &[String],
) -> Result<(), PolicyRoutingError> {
    // For now, only handle default route
    if prefix == "0.0.0.0/0" || prefix == "default" {
        if let Ok(peer_id) = Uuid::parse_str(active_peer_id) {
            return set_exit_node(&peer_id, None); // Load config if needed
        }
    }
    
    // TODO: Handle other overlapping prefixes in the future
    log::debug!("set_active_peer_for_prefix not yet implemented for prefix: {}", prefix);
    Ok(())
}

// Remove peer routing table and clean up
// Remove peer routing table (public wrapper - loads config internally if needed)
pub fn remove_peer_routing_table(peer_id: &Uuid, table_id: u32) -> Result<(), PolicyRoutingError> {
    remove_peer_routing_table_impl(peer_id, table_id, None)
}

// Internal implementation that accepts network reference to avoid deadlock
pub fn remove_peer_routing_table_impl(peer_id: &Uuid, table_id: u32, network: Option<&Network>) -> Result<(), PolicyRoutingError> {
    let peer_id_str = peer_id.to_string();
    
    log::info!("Removing routing table {} for peer {}", table_id, peer_id_str);
    
    // Flush all routes from the table
    let flush_cmd = &["ip", "route", "flush", "table", &table_id.to_string()];
    if let Err(e) = shell_cmd(flush_cmd) {
        log::warn!("Failed to flush table {}: {} (continuing anyway)", table_id, e);
        // Continue with cleanup even if flush fails
    }
    
    // Remove PBR rules for this peer
    if let Err(e) = remove_pbr_rules_for_peer(peer_id, table_id) {
        log::warn!("Failed to remove PBR rules for peer {}: {} (continuing anyway)", peer_id_str, e);
    }
    
    // Remove from persisted state and handle exit node
    let mut state = load_mode_state()
        .map_err(|e| PolicyRoutingError::PersistenceError(format!("Failed to load mode state: {}", e)))?
        .ok_or_else(|| PolicyRoutingError::PersistenceError("No mode state found".to_string()))?;
    
    // Check if this peer was the exit node
    if let Some(prefix_state) = state.prefix_active_backup.get("0.0.0.0/0") {
        if prefix_state.active_peer_id == peer_id_str {
            // This peer was the exit node, remove exit node rules
            log::info!("Removing exit node rules for peer {}", peer_id_str);
            
            // Parse rules once and remove exit node rules
            if let Ok(rules) = get_ip_rules_cached() {
                for rule in &rules {
                    if rule.to == Some("0.0.0.0/0".to_string())
                        && rule.table_id == Some(table_id)
                        && rule.priority >= 20000 {
                        let priority_str = rule.priority.to_string();
                        let _ = shell_cmd(&["ip", "rule", "del", "priority", &priority_str]);
                        log::info!("Removed exit node rule with priority {}", rule.priority);
                    }
                }
            }
            
            // Remove LAN exception rules if they exist (supports multiple comma-separated CIDRs)
            if let Some(lan_cidr_str) = &state.lan_cidr {
                let lan_cidrs = parse_lan_cidrs(lan_cidr_str);
                let lan_interface = find_lan_interface().ok();
                
                for lan_cidr in &lan_cidrs {
                    if let Some(ref iface) = lan_interface {
                        // Remove eth0 LAN exception rule
                        let _ = shell_cmd(&[
                            "ip", "rule", "del",
                            "iif", iface,
                            "to", lan_cidr,
                            "lookup", "main",
                        ]);
                        log::info!("Removed LAN exception rule for {}", lan_cidr);
                    }
                    
                    // Remove WireGuard peer LAN exception rules
                    // Get WireGuard interface from network if available
                    if let Some(network_ref) = network {
                        let wg_interface = &network_ref.name;
                        let wg_subnet = network_ref.subnet.to_string();
                        // Remove old subnet-wide rule (migration)
                        let _ = shell_cmd(&[
                            "ip", "rule", "del",
                            "from", &wg_subnet,
                            "iif", wg_interface,
                            "to", lan_cidr,
                            "lookup", "main",
                        ]);
                        // Remove per-peer LAN exception rules
                        for (pid, p) in &network_ref.peers {
                            if *pid == network_ref.this_peer {
                                continue;
                            }
                            let peer_addr = format!("{}/32", p.address);
                            let _ = shell_cmd(&[
                                "ip", "rule", "del",
                                "from", &peer_addr,
                                "iif", wg_interface,
                                "to", lan_cidr,
                                "lookup", "main",
                            ]);
                        }
                        log::info!("Removed WireGuard peer LAN exception rules for {}", lan_cidr);
                    }
                }
            }
            
            // This peer was the exit node, remove it from state
            state.prefix_active_backup.remove("0.0.0.0/0");
            // Try to find a new exit node from remaining peers
            if let Some(network_ref) = network {
                // Use provided network reference
                let peers_with_default = get_peers_with_default_route(network_ref);
                // Filter out the peer being deleted
                let remaining_peers: Vec<Uuid> = peers_with_default
                    .into_iter()
                    .filter(|&p| p != *peer_id)
                    .collect();
                if let Some(new_exit_node) = remaining_peers.first() {
                    log::info!("Selecting new exit node: {}", new_exit_node);
                    if let Err(e) = set_exit_node(new_exit_node, Some(network_ref)) {
                        log::warn!("Failed to set new exit node: {}", e);
                    }
                } else {
                    log::info!("No other peers with default route, exit node removed");
                }
            } else {
                // Load config if network not provided (for backward compatibility)
                let config = crate::conf::util::get_config()
                    .map_err(|e| PolicyRoutingError::PersistenceError(format!("Failed to load config: {}", e)))?;
                let peers_with_default = get_peers_with_default_route(&config.network);
                // Filter out the peer being deleted
                let remaining_peers: Vec<Uuid> = peers_with_default
                    .into_iter()
                    .filter(|&p| p != *peer_id)
                    .collect();
                if let Some(new_exit_node) = remaining_peers.first() {
                    log::info!("Selecting new exit node: {}", new_exit_node);
                    if let Err(e) = set_exit_node(new_exit_node, Some(&config.network)) {
                        log::warn!("Failed to set new exit node: {}", e);
                    }
                } else {
                    log::info!("No other peers with default route, exit node removed");
                }
            }
        }
    }
    
    state.peer_table_ids.remove(&peer_id_str);
    save_mode_state(&state)
        .map_err(|e| PolicyRoutingError::PersistenceError(format!("Failed to save mode state: {}", e)))?;
    
    log::info!("Successfully removed routing table {} for peer {}", table_id, peer_id_str);
    Ok(())
}

// Get table ID for a peer
pub fn get_peer_table_id(peer_id: &Uuid) -> Result<Option<u32>, PolicyRoutingError> {
    let peer_id_str = peer_id.to_string();
    let state = match load_mode_state()
        .map_err(|e| PolicyRoutingError::PersistenceError(format!("Failed to load mode state: {}", e)))?
    {
        Some(s) => s,
        None => return Ok(None),
    };
    
    Ok(state.peer_table_ids.get(&peer_id_str).copied())
}

// Get all routes for a peer from connections
// Returns a deduplicated list of CIDR strings including:
// 1. Routes the peer can reach through connections
// 2. The peer's own address/32 route
pub fn get_peer_advertised_routes(peer_id: &Uuid, network: &Network) -> Vec<String> {
    log::debug!("[get_peer_advertised_routes] Getting routes for peer {}", peer_id);
    let mut routes = std::collections::HashSet::new();
    
    // First, add the peer's own address/32 route
    if let Some(peer) = network.peers.get(peer_id) {
        let peer_address = format!("{}/32", peer.address);
        log::debug!("[get_peer_advertised_routes] Adding peer's own address: {}", peer_address);
        routes.insert(peer_address);
    } else {
        log::warn!("[get_peer_advertised_routes] Peer {} not found in network peers", peer_id);
    }
    
    // Iterate through all connections to get routes the peer can reach
    log::debug!("[get_peer_advertised_routes] Checking {} connections...", network.connections.len());
    for (connection_id, connection) in &network.connections {
        // Check if this peer is involved in the connection
        if connection_id.a == *peer_id {
            // This peer is peer A, so routes it can reach are in allowed_ips_b_to_a
            // (what peer B advertises to peer A)
            log::debug!("[get_peer_advertised_routes] Peer {} is peer A in connection {}", peer_id, connection_id);
            for cidr in &connection.allowed_ips_b_to_a {
                log::debug!("[get_peer_advertised_routes] Adding route from allowed_ips_b_to_a: {}", cidr);
                routes.insert(cidr.to_string());
            }
        } else if connection_id.b == *peer_id {
            // This peer is peer B, so routes it can reach are in allowed_ips_a_to_b
            // (what peer A advertises to peer B)
            log::debug!("[get_peer_advertised_routes] Peer {} is peer B in connection {}", peer_id, connection_id);
            for cidr in &connection.allowed_ips_a_to_b {
                log::debug!("[get_peer_advertised_routes] Adding route from allowed_ips_a_to_b: {}", cidr);
                routes.insert(cidr.to_string());
            }
        }
    }
    
    let mut result: Vec<String> = routes.into_iter().collect();
    result.sort();
    log::info!("[get_peer_advertised_routes] Returning {} routes for peer {}: {:?}", result.len(), peer_id, result);
    result
}

// Update routes for a peer (flush old routes and install new ones)
// This is called when connections are modified
pub fn update_peer_routes(peer_id: &Uuid, network: &Network, wg_interface: &str) -> Result<(), PolicyRoutingError> {
    // Get the table ID for this peer
    let table_id = match get_peer_table_id(peer_id)? {
        Some(id) => id,
        None => {
            log::debug!("No routing table found for peer {}, skipping route update", peer_id);
            return Ok(());
        }
    };
    
    // Flush existing routes from the table
    let flush_cmd = &["ip", "route", "flush", "table", &table_id.to_string()];
    if let Err(e) = shell_cmd(flush_cmd) {
        log::warn!("Failed to flush table {} for peer {}: {} (continuing anyway)", table_id, peer_id, e);
    }
    
    // Get updated routes
    let routes = get_peer_advertised_routes(peer_id, network);
    
    // Install new routes
    install_peer_routes(peer_id, table_id, &routes, wg_interface)?;
    
    // Update PBR rules for this peer
    let lan_interface = find_lan_interface()?;
    if let Err(e) = update_pbr_rules_for_peer(peer_id, network, &lan_interface) {
        log::warn!("Failed to update PBR rules for peer {}: {} (continuing anyway)", peer_id, e);
    }
    
    // Check if this peer has default route and handle exit node logic
    if routes.contains(&"0.0.0.0/0".to_string()) || routes.contains(&"default".to_string()) {
        // If no exit node is set, set this peer as exit node
        // Pass network to avoid deadlock
        if get_exit_node()?.is_none() {
            log::info!("No exit node set, setting peer {} as exit node", peer_id);
            if let Err(e) = set_exit_node(peer_id, Some(network)) {
                log::warn!("Failed to set peer {} as exit node: {}", peer_id, e);
            }
        }
    }
    
    // Ensure LAN access rule exists for this peer (if an exit node is set)
    // This ensures new peers get their LAN access rule immediately
    if let Ok(Some(_exit_node)) = get_exit_node() {
        if let Err(e) = ensure_peer_lan_access_rule(peer_id, network) {
            log::warn!("Failed to ensure LAN access rule for peer {}: {} (continuing anyway)", peer_id, e);
        }
    }
    
    log::info!("Updated routes for peer {} in table {} ({} routes)", peer_id, table_id, routes.len());
    Ok(())
}

/// Set LAN access for a specific peer
/// Returns the new LAN access state
pub fn set_peer_lan_access(peer_id: &Uuid, has_lan_access: bool, network: &Network) -> Result<bool, PolicyRoutingError> {
    let peer_id_str = peer_id.to_string();
    
    // Load current state
    let mut state = load_mode_state()
        .map_err(|e| PolicyRoutingError::PersistenceError(format!("Failed to load mode state: {}", e)))?
        .ok_or_else(|| PolicyRoutingError::PersistenceError("No mode state found".to_string()))?;
    
    // Update the peer's LAN access setting
    state.peer_lan_access.insert(peer_id_str.clone(), has_lan_access);
    
    // Save the state
    save_mode_state(&state)
        .map_err(|e| PolicyRoutingError::PersistenceError(format!("Failed to save mode state: {}", e)))?;
    
    // Get peer info
    let peer = network.peers.get(peer_id)
        .ok_or_else(|| PolicyRoutingError::PersistenceError(format!("Peer {} not found", peer_id)))?;
    
    let peer_addr = format!("{}/32", peer.address);
    let wg_interface = &network.name;
    
    // Get LAN CIDRs from state (supports multiple comma-separated CIDRs)
    let lan_cidr_str = state.lan_cidr
        .ok_or_else(|| PolicyRoutingError::PersistenceError("No LAN CIDR configured".to_string()))?;
    let lan_cidrs = parse_lan_cidrs(&lan_cidr_str);
    
    if has_lan_access {
        // Add the LAN access rule for this peer
        // Use a priority based on the peer's position (find index in sorted peers)
        let mut peer_index = 0u32;
        for (pid, _) in &network.peers {
            if *pid == network.this_peer {
                continue;
            }
            if *pid == *peer_id {
                break;
            }
            peer_index += 1;
        }
        
        // Calculate priority (19899 - 100 = 19799 base, + peer_index)
        let exception_priority = 20000 - 1; // eth0 exception priority
        let wg_peer_lan_base_priority = exception_priority - 100;
        
        for (cidr_idx, lan_cidr) in lan_cidrs.iter().enumerate() {
            // First remove any existing rule for this peer
            let _ = shell_cmd(&[
                "ip", "rule", "del",
                "from", &peer_addr,
                "iif", wg_interface,
                "to", lan_cidr,
                "lookup", "main",
            ]);
            
            // Use unique priority: base + (cidr_index * 100) + peer_index
            let peer_priority = wg_peer_lan_base_priority + (cidr_idx as u32 * 100) + peer_index;
            let peer_priority_str = peer_priority.to_string();
            
            // Add the rule
            let cmd = &[
                "ip", "rule", "add",
                "from", &peer_addr,
                "iif", wg_interface,
                "to", lan_cidr,
                "lookup", "main",
                "priority", &peer_priority_str,
            ];
            
            if let Err(e) = shell_cmd(cmd) {
                log::warn!("Failed to add LAN access rule for peer {} ({}) to {}: {}", peer.name, peer_addr, lan_cidr, e);
            } else {
                log::info!("Added LAN access rule for peer {} ({}) to {}: -> main table (priority {})", 
                    peer.name, peer_addr, lan_cidr, peer_priority);
            }
        }
    } else {
        // Remove the LAN access rule for this peer (all CIDRs)
        for lan_cidr in &lan_cidrs {
            let _ = shell_cmd(&[
                "ip", "rule", "del",
                "from", &peer_addr,
                "iif", wg_interface,
                "to", lan_cidr,
                "lookup", "main",
            ]);
        }
        log::info!("Removed LAN access rules for peer {} ({})", peer.name, peer_addr);
    }
    
    Ok(has_lan_access)
}

/// Ensure LAN access rule exists for a peer based on its current setting
/// This is called when adding new peers to ensure they get their LAN access rule
fn ensure_peer_lan_access_rule(peer_id: &Uuid, network: &Network) -> Result<(), PolicyRoutingError> {
    let peer_id_str = peer_id.to_string();
    
    // Skip the router itself
    if *peer_id == network.this_peer {
        return Ok(());
    }
    
    // Load current state to get LAN access setting and LAN CIDR
    let state = load_mode_state()
        .map_err(|e| PolicyRoutingError::PersistenceError(format!("Failed to load mode state: {}", e)))?
        .ok_or_else(|| PolicyRoutingError::PersistenceError("No mode state found".to_string()))?;
    
    // Get LAN CIDRs (supports multiple comma-separated CIDRs)
    let lan_cidr_str = match &state.lan_cidr {
        Some(cidr) => cidr.clone(),
        None => return Ok(()), // No LAN CIDR configured, nothing to do
    };
    let lan_cidrs = parse_lan_cidrs(&lan_cidr_str);
    
    // Check if peer has LAN access (default is true)
    let has_lan_access = state.peer_lan_access
        .get(&peer_id_str)
        .copied()
        .unwrap_or(true);
    
    // Get peer info
    let peer = network.peers.get(peer_id)
        .ok_or_else(|| PolicyRoutingError::PersistenceError(format!("Peer {} not found", peer_id)))?;
    
    let peer_addr = format!("{}/32", peer.address);
    let wg_interface = &network.name;
    
    if has_lan_access {
        // Find peer index for priority calculation
        let mut peer_index = 0u32;
        for (pid, _) in &network.peers {
            if *pid == network.this_peer {
                continue;
            }
            if *pid == *peer_id {
                break;
            }
            peer_index += 1;
        }
        
        let exception_priority = 20000 - 1;
        let wg_peer_lan_base_priority = exception_priority - 100;
        
        for (cidr_idx, lan_cidr) in lan_cidrs.iter().enumerate() {
            // First remove any existing rule for this peer
            let _ = shell_cmd(&[
                "ip", "rule", "del",
                "from", &peer_addr,
                "iif", wg_interface,
                "to", lan_cidr,
                "lookup", "main",
            ]);
            
            // Use unique priority: base + (cidr_index * 100) + peer_index
            let peer_priority = wg_peer_lan_base_priority + (cidr_idx as u32 * 100) + peer_index;
            let peer_priority_str = peer_priority.to_string();
            
            // Add the rule
            let cmd = &[
                "ip", "rule", "add",
                "from", &peer_addr,
                "iif", wg_interface,
                "to", lan_cidr,
                "lookup", "main",
                "priority", &peer_priority_str,
            ];
            
            if let Err(e) = shell_cmd(cmd) {
                log::warn!("Failed to ensure LAN access rule for peer {} ({}) to {}: {}", peer.name, peer_addr, lan_cidr, e);
            } else {
                log::debug!("Ensured LAN access rule for peer {} ({}) to {}: -> main table (priority {})", 
                    peer.name, peer_addr, lan_cidr, peer_priority);
            }
        }
    }
    // If no LAN access, we don't remove the rule here - that's handled by set_peer_lan_access
    
    Ok(())
}

/// Get LAN access status for all peers
pub fn get_all_peer_lan_access() -> Result<HashMap<String, bool>, PolicyRoutingError> {
    let state = load_mode_state()
        .map_err(|e| PolicyRoutingError::PersistenceError(format!("Failed to load mode state: {}", e)))?;
    
    match state {
        Some(s) => Ok(s.peer_lan_access),
        None => Ok(HashMap::new()),
    }
}

