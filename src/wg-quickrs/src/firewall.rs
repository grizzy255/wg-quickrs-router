// Firewall module: Add NAT + forwarding rules on Router Mode;
// remove them on Host Mode
//
// Responsibilities:
// - STEP 2: Configure firewall rules for NAT/MASQUERADE and forwarding

use crate::helpers::{shell_cmd, parse_lan_cidrs};
use crate::conf::util::get_config;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FirewallError {
    #[error("Firewall utility error: {0}")]
    UtilityError(String),
    #[error("NAT rule error: {0}")]
    NatRuleError(String),
    #[error("Forwarding rule error: {0}")]
    ForwardingRuleError(String),
    #[error("Config error: {0}")]
    ConfigError(String),
}

// Enable Router Mode firewall rules
// Adds NAT/MASQUERADE and forwarding rules for LAN -> WireGuard interface
// Supports multiple comma-separated CIDRs (e.g., "192.168.1.0/24,10.0.0.0/8")
pub fn enable_router_mode_firewall(lan_cidr: &str) -> Result<(), FirewallError> {
    // Get config first
    let config = get_config()
        .map_err(|e| FirewallError::ConfigError(format!("Failed to load config: {}", e)))?;
    
    // Get LAN CIDR from parameter or config
    let cidr_str = if lan_cidr.is_empty() {
        // Try to get from config
        match &config.agent.router.lan_cidr {
            Some(cidr) => cidr.clone(),
            None => {
                return Err(FirewallError::ConfigError("LAN CIDR is required for Router Mode firewall rules".to_string()));
            }
        }
    } else {
        lan_cidr.to_string()
    };
    
    let cidrs = parse_lan_cidrs(&cidr_str);
    if cidrs.is_empty() {
        return Err(FirewallError::ConfigError("No valid LAN CIDRs provided".to_string()));
    }
    
    log::info!("Enabling Router Mode firewall rules for LAN CIDRs: {:?}", cidrs);
    
    let wg_interface = &config.network.name;
    
    // Determine LAN interface from first CIDR (assume same interface for all)
    let lan_interface = find_lan_interface(&cidrs[0])?;
    
    log::info!("LAN interface: {}, WireGuard interface: {}", lan_interface, wg_interface);
    
    // Check if iptables is available
    if shell_cmd(&["iptables", "--version"]).is_err() {
        return Err(FirewallError::UtilityError("iptables not available".to_string()));
    }
    
    // Add NAT/MASQUERADE rules for each CIDR
    for cidr in &cidrs {
        let masq_cmd = &[
            "iptables", "-t", "nat", "-C", "POSTROUTING",
            "-s", cidr,
            "-o", wg_interface,
            "-j", "MASQUERADE"
        ];
        
        // Check if rule already exists
        let rule_exists = shell_cmd(masq_cmd).is_ok();
        
        if !rule_exists {
            let add_masq_cmd = &[
                "iptables", "-t", "nat", "-A", "POSTROUTING",
                "-s", cidr,
                "-o", wg_interface,
                "-j", "MASQUERADE"
            ];
            
            if let Err(e) = shell_cmd(add_masq_cmd) {
                return Err(FirewallError::NatRuleError(format!("Failed to add MASQUERADE rule for {}: {}", cidr, e)));
            }
            log::info!("Added NAT/MASQUERADE rule: {} -> {}", cidr, wg_interface);
        } else {
            log::debug!("NAT/MASQUERADE rule already exists for {}", cidr);
        }
    }
    
    // Add NAT/MASQUERADE rule for WireGuard peers: NAT traffic from WireGuard subnet going out WireGuard interface
    // This allows WireGuard peers to use the exit node for internet traffic
    let wg_subnet = config.network.subnet.to_string();
    let wg_peer_masq_cmd = &[
        "iptables", "-t", "nat", "-C", "POSTROUTING",
        "-s", &wg_subnet,
        "-o", wg_interface,
        "-j", "MASQUERADE"
    ];
    
    let wg_peer_rule_exists = shell_cmd(wg_peer_masq_cmd).is_ok();
    
    if !wg_peer_rule_exists {
        let add_wg_peer_masq_cmd = &[
            "iptables", "-t", "nat", "-A", "POSTROUTING",
            "-s", &wg_subnet,
            "-o", wg_interface,
            "-j", "MASQUERADE"
        ];
        
        if let Err(e) = shell_cmd(add_wg_peer_masq_cmd) {
            log::warn!("Failed to add MASQUERADE rule for WireGuard peers ({} -> {}): {} (continuing anyway)", wg_subnet, wg_interface, e);
        } else {
            log::info!("Added NAT/MASQUERADE rule for WireGuard peers: {} -> {}", wg_subnet, wg_interface);
        }
    } else {
        log::debug!("NAT/MASQUERADE rule for WireGuard peers already exists");
    }
    
    // Add forwarding rules: Allow traffic from LAN to WireGuard
    let fwd_in_cmd = &[
        "iptables", "-C", "FORWARD",
        "-i", &lan_interface,
        "-o", wg_interface,
        "-j", "ACCEPT"
    ];
    
    if shell_cmd(fwd_in_cmd).is_err() {
        let add_fwd_in_cmd = &[
            "iptables", "-A", "FORWARD",
            "-i", &lan_interface,
            "-o", wg_interface,
            "-j", "ACCEPT"
        ];
        
        if let Err(e) = shell_cmd(add_fwd_in_cmd) {
            return Err(FirewallError::ForwardingRuleError(format!("Failed to add forwarding rule (LAN->WG): {}", e)));
        }
        log::info!("Added forwarding rule: {} -> {}", lan_interface, wg_interface);
    }
    
    // Add forwarding rules: Allow return traffic from WireGuard to LAN
    let fwd_out_cmd = &[
        "iptables", "-C", "FORWARD",
        "-i", wg_interface,
        "-o", &lan_interface,
        "-j", "ACCEPT"
    ];
    
    if shell_cmd(fwd_out_cmd).is_err() {
        let add_fwd_out_cmd = &[
            "iptables", "-A", "FORWARD",
            "-i", wg_interface,
            "-o", &lan_interface,
            "-j", "ACCEPT"
        ];
        
        if let Err(e) = shell_cmd(add_fwd_out_cmd) {
            return Err(FirewallError::ForwardingRuleError(format!("Failed to add forwarding rule (WG->LAN): {}", e)));
        }
        log::info!("Added forwarding rule: {} -> {}", wg_interface, lan_interface);
    }
    
    // Add MSS clamping rules to fix MTU issues through WireGuard tunnel
    // This prevents "some sites don't load" issues caused by large TCP segments
    // WireGuard MTU is typically 1420, so TCP MSS needs to be clamped to fit
    
    // MSS clamp for traffic going OUT to WireGuard (FORWARD chain)
    let mss_out_check = &[
        "iptables", "-t", "mangle", "-C", "FORWARD",
        "-p", "tcp", "--tcp-flags", "SYN,RST", "SYN",
        "-o", wg_interface,
        "-j", "TCPMSS", "--clamp-mss-to-pmtu"
    ];
    
    if shell_cmd(mss_out_check).is_err() {
        let mss_out_cmd = &[
            "iptables", "-t", "mangle", "-A", "FORWARD",
            "-p", "tcp", "--tcp-flags", "SYN,RST", "SYN",
            "-o", wg_interface,
            "-j", "TCPMSS", "--clamp-mss-to-pmtu"
        ];
        if let Err(e) = shell_cmd(mss_out_cmd) {
            log::warn!("Failed to add MSS clamping rule (outgoing): {} (non-fatal)", e);
        } else {
            log::info!("Added MSS clamping rule: outgoing TCP SYN -> {} (clamp to PMTU)", wg_interface);
        }
    }
    
    // MSS clamp for traffic coming IN from WireGuard (FORWARD chain)
    let mss_in_check = &[
        "iptables", "-t", "mangle", "-C", "FORWARD",
        "-p", "tcp", "--tcp-flags", "SYN,RST", "SYN",
        "-i", wg_interface,
        "-j", "TCPMSS", "--clamp-mss-to-pmtu"
    ];
    
    if shell_cmd(mss_in_check).is_err() {
        let mss_in_cmd = &[
            "iptables", "-t", "mangle", "-A", "FORWARD",
            "-p", "tcp", "--tcp-flags", "SYN,RST", "SYN",
            "-i", wg_interface,
            "-j", "TCPMSS", "--clamp-mss-to-pmtu"
        ];
        if let Err(e) = shell_cmd(mss_in_cmd) {
            log::warn!("Failed to add MSS clamping rule (incoming): {} (non-fatal)", e);
        } else {
            log::info!("Added MSS clamping rule: incoming TCP SYN <- {} (clamp to PMTU)", wg_interface);
        }
    }
    
    // MSS clamp in POSTROUTING for locally-originated traffic
    let mss_post_check = &[
        "iptables", "-t", "mangle", "-C", "POSTROUTING",
        "-p", "tcp", "--tcp-flags", "SYN,RST", "SYN",
        "-o", wg_interface,
        "-j", "TCPMSS", "--clamp-mss-to-pmtu"
    ];
    
    if shell_cmd(mss_post_check).is_err() {
        let mss_post_cmd = &[
            "iptables", "-t", "mangle", "-A", "POSTROUTING",
            "-p", "tcp", "--tcp-flags", "SYN,RST", "SYN",
            "-o", wg_interface,
            "-j", "TCPMSS", "--clamp-mss-to-pmtu"
        ];
        if let Err(e) = shell_cmd(mss_post_cmd) {
            log::warn!("Failed to add MSS clamping rule (postrouting): {} (non-fatal)", e);
        } else {
            log::info!("Added MSS clamping rule: POSTROUTING TCP SYN -> {} (clamp to PMTU)", wg_interface);
        }
    }
    
    log::info!("Successfully enabled Router Mode firewall rules");
    Ok(())
}

// Disable Router Mode firewall rules
pub fn disable_router_mode_firewall() -> Result<(), FirewallError> {
    log::info!("Disabling Router Mode firewall rules");
    
    // Get config to find interfaces
    let config = match get_config() {
        Ok(c) => c,
        Err(_) => {
            log::warn!("Failed to load config, attempting to remove rules by pattern");
            // Try to remove rules without config
            remove_firewall_rules_by_pattern()?;
            return Ok(());
        }
    };
    
    let wg_interface = &config.network.name;
    
    // Get LAN CIDRs from persisted state (supports multiple comma-separated CIDRs)
    let lan_cidr_str = match crate::mode::persist::load_mode_state() {
        Ok(Some(state)) => state.lan_cidr,
        _ => {
            log::warn!("No persisted state found, attempting pattern-based removal");
            remove_firewall_rules_by_pattern()?;
            return Ok(());
        }
    };
    
    let lan_cidr_str = match lan_cidr_str {
        Some(cidr) => cidr,
        None => {
            log::warn!("No LAN CIDR in state, attempting pattern-based removal");
            remove_firewall_rules_by_pattern()?;
            return Ok(());
        }
    };
    
    let cidrs = parse_lan_cidrs(&lan_cidr_str);
    if cidrs.is_empty() {
        log::warn!("No valid LAN CIDRs found, attempting pattern-based removal");
        remove_firewall_rules_by_pattern()?;
        return Ok(());
    }
    
    let lan_interface = find_lan_interface(&cidrs[0])?;
    
    // Remove MASQUERADE rules for each CIDR
    for cidr in &cidrs {
        let del_masq_cmd = &[
            "iptables", "-t", "nat", "-D", "POSTROUTING",
            "-s", cidr,
            "-o", wg_interface,
            "-j", "MASQUERADE"
        ];
        
        if shell_cmd(del_masq_cmd).is_ok() {
            log::info!("Removed NAT/MASQUERADE rule for {}", cidr);
        } else {
            log::debug!("MASQUERADE rule not found for {} (may have been removed already)", cidr);
        }
    }
    
    // Remove forwarding rules (only need to remove once, not per-CIDR)
    let del_fwd_in_cmd = &[
        "iptables", "-D", "FORWARD",
        "-i", &lan_interface,
        "-o", wg_interface,
        "-j", "ACCEPT"
    ];
    
    if shell_cmd(del_fwd_in_cmd).is_ok() {
        log::info!("Removed forwarding rule (LAN->WG)");
    }
    
    let del_fwd_out_cmd = &[
        "iptables", "-D", "FORWARD",
        "-i", wg_interface,
        "-o", &lan_interface,
        "-j", "ACCEPT"
    ];
    
    if shell_cmd(del_fwd_out_cmd).is_ok() {
        log::info!("Removed forwarding rule (WG->LAN)");
    }
    
    // Remove MSS clamping rules
    let del_mss_out = &[
        "iptables", "-t", "mangle", "-D", "FORWARD",
        "-p", "tcp", "--tcp-flags", "SYN,RST", "SYN",
        "-o", wg_interface,
        "-j", "TCPMSS", "--clamp-mss-to-pmtu"
    ];
    if shell_cmd(del_mss_out).is_ok() {
        log::info!("Removed MSS clamping rule (outgoing)");
    }
    
    let del_mss_in = &[
        "iptables", "-t", "mangle", "-D", "FORWARD",
        "-p", "tcp", "--tcp-flags", "SYN,RST", "SYN",
        "-i", wg_interface,
        "-j", "TCPMSS", "--clamp-mss-to-pmtu"
    ];
    if shell_cmd(del_mss_in).is_ok() {
        log::info!("Removed MSS clamping rule (incoming)");
    }
    
    let del_mss_post = &[
        "iptables", "-t", "mangle", "-D", "POSTROUTING",
        "-p", "tcp", "--tcp-flags", "SYN,RST", "SYN",
        "-o", wg_interface,
        "-j", "TCPMSS", "--clamp-mss-to-pmtu"
    ];
    if shell_cmd(del_mss_post).is_ok() {
        log::info!("Removed MSS clamping rule (postrouting)");
    }
    
    log::info!("Successfully disabled Router Mode firewall rules");
    Ok(())
}

// Helper: Find LAN interface by matching CIDR
fn find_lan_interface(lan_cidr: &str) -> Result<String, FirewallError> {
    // Extract network from CIDR (e.g., "192.168.1.0/24" -> "192.168.1")
    let parts: Vec<&str> = lan_cidr.split('/').collect();
    if parts.len() != 2 {
        return Err(FirewallError::ConfigError(format!("Invalid CIDR format: {}", lan_cidr)));
    }
    
    let network = parts[0];
    let network_parts: Vec<&str> = network.split('.').collect();
    if network_parts.len() < 3 {
        return Err(FirewallError::ConfigError(format!("Invalid network address: {}", network)));
    }
    
    // Extract first 3 octets for matching
    let network_prefix = format!("{}.{}.{}", network_parts[0], network_parts[1], network_parts[2]);
    
    // List all interfaces and find one with matching IP
    let ip_output = shell_cmd(&["ip", "-4", "addr", "show"])
        .map_err(|e| FirewallError::ConfigError(format!("Failed to list interfaces: {}", e)))?;
    
    let ip_output_str = String::from_utf8_lossy(&ip_output.stdout);
    let mut current_interface: Option<String> = None;
    
    // Parse ip addr show output to find interface with matching network
    for line in ip_output_str.lines() {
        // Interface line: "2: eth0@if56: <BROADCAST,MULTICAST,UP,LOWER_UP>"
        if line.contains(':') && !line.starts_with("    ") && !line.starts_with(" ") {
            let iface_part = line.split(':').nth(1);
            if let Some(iface) = iface_part {
                let iface_name = iface.split('@').next().unwrap_or("").trim();
                if !iface_name.is_empty() && iface_name != "lo" {
                    current_interface = Some(iface_name.to_string());
                }
            }
        }
        // IP line: "    inet 192.168.1.198/24 ..."
        else if let Some(iface) = &current_interface {
            if line.contains("inet") && line.contains(&network_prefix) {
                log::debug!("Found LAN interface: {} for CIDR {}", iface, lan_cidr);
                return Ok(iface.clone());
            }
        }
    }
    
    // Fallback: try common interface names
    for iface in &["eth0", "ens3", "enp0s3", "enp1s0"] {
        if shell_cmd(&["ip", "addr", "show", iface]).is_ok() {
            log::debug!("Using fallback LAN interface: {} for CIDR {}", iface, lan_cidr);
            return Ok(iface.to_string());
        }
    }
    
    // Default to eth0
    log::warn!("Could not determine LAN interface for CIDR {}, defaulting to eth0", lan_cidr);
    Ok("eth0".to_string())
}

// Helper: Remove firewall rules by pattern (fallback when config unavailable)
fn remove_firewall_rules_by_pattern() -> Result<(), FirewallError> {
    // Try to remove MASQUERADE rules that match our pattern
    // This is a best-effort cleanup
    log::debug!("Attempting pattern-based firewall rule removal");
    Ok(())
}

