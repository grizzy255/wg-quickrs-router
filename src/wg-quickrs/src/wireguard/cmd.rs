use crate::{conf, mode};
use once_cell::sync::Lazy;
use wg_quickrs_lib::helpers::get_peer_wg_config;
use wg_quickrs_lib::types::config::{Config};
use wg_quickrs_lib::types::api::{Telemetry, TelemetryData, TelemetryDatum};
use wg_quickrs_lib::types::misc::{WireGuardStatus};
use std::collections::{BTreeMap, VecDeque};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};
use chrono::Utc;
use tempfile::NamedTempFile;
use thiserror::Error;
use tokio::signal::unix::{signal, SignalKind};
use wg_quickrs_lib::types::network::ConnectionId;
use crate::helpers::{shell_cmd, ShellError};
use crate::wireguard::wg_quick;

const TELEMETRY_CAPACITY: usize = 21;
const TELEMETRY_INTERVAL: u64 = 1000;
type TelemetryType = Lazy<Arc<RwLock<VecDeque<TelemetryData>>>>;
static TELEMETRY: TelemetryType =
    Lazy::new(|| Arc::new(RwLock::new(VecDeque::with_capacity(TELEMETRY_CAPACITY))));

static LAST_TELEMETRY_QUERY_TS: Lazy<Arc<RwLock<u64>>> = Lazy::new(|| Arc::new(RwLock::new(0)));

fn update_timestamp(ts: &Arc<RwLock<u64>>) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let mut ts = ts.write().unwrap();
    *ts = now;
}

fn get_since_timestamp(ts: &Arc<RwLock<u64>>) -> u64 {
    let start = *ts.read().unwrap();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    now.saturating_sub(start) * 1000
}

#[derive(Error, Debug)]
pub enum WireGuardCommandError {
    #[error("failed to acquire lock: {0}")]
    MutexLockFailed(String),
    #[error("WireGuard interface does not exist")]
    InterfaceMissing,
    #[error("{0}")]
    ShellError(#[from] ShellError),
    #[error("failed to write file at {0} failed: {1}")]
    FileWriteError(PathBuf, std::io::Error),
    #[error("failed to sync WireGuard interface")]
    InterfaceSyncFailed(),
    #[error("tunnel operation failed: {0}")]
    TunnelError(#[from] wg_quick::TunnelError),
}

static WG_TUNNEL_MANAGER: Lazy<RwLock<wg_quick::TunnelManager>> = Lazy::new(|| RwLock::new(wg_quick::TunnelManager::new(Default::default())));
pub static WG_STATUS: RwLock<WireGuardStatus> = RwLock::new(WireGuardStatus::UNKNOWN);

pub(crate) async fn run_vpn_server(
    config: &Config,
) -> std::io::Result<()> {
    if !config.agent.vpn.enabled {
        log::warn!("WireGuard tunnel is disabled");
        // Set status to DOWN when VPN is disabled (not UNKNOWN)
        *WG_STATUS
            .write()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to set status: {}", e)))? =
            WireGuardStatus::DOWN;
        return Ok(());
    }
    let mut tunnel_manager = WG_TUNNEL_MANAGER.write().unwrap();
    tunnel_manager.config = Some(config.clone());
    drop(tunnel_manager);

    Box::pin(async move {
        let _ = disable_tunnel();

        log::info!("Starting WireGuard tunnel...");
        enable_tunnel().unwrap_or_else(|e| {
            log::error!("Failed to enable the wireguard tunnel: {e}");
        });

        let mut signal_terminate = signal(SignalKind::terminate()).unwrap();
        let mut signal_interrupt = signal(SignalKind::interrupt()).unwrap();
        let mut ticker = tokio::time::interval(Duration::from_millis(TELEMETRY_INTERVAL));

        tokio::select! {
            _ = async {
                loop {
                    ticker.tick().await;
                    run_loop();
                }
            } => {},
            _ = signal_terminate.recv() => log::info!("Received SIGTERM"),
            _ = signal_interrupt.recv() => log::info!("Received SIGINT"),
        }

        let _ = disable_tunnel();
        Ok(())
    })
        .await
}

fn run_loop() {
    match WG_STATUS.read() {
        Ok(status) => {
            if status.clone() != WireGuardStatus::UP {
                return;
            }
        }
        Err(e) => {
            log::error!("{}", WireGuardCommandError::MutexLockFailed(e.to_string()));
            return;
        }
    }

    if get_since_timestamp(&LAST_TELEMETRY_QUERY_TS)
        > TELEMETRY_INTERVAL * TELEMETRY_CAPACITY as u64
    {
        return;
    }

    let config = match conf::util::get_config() {
        Ok(config) => config,
        Err(e) => {
            log::error!("{e}");
            return;
        }
    };

    match show_dump(&config) {
        Ok(telemetry) => {
            let mut buf = TELEMETRY.write().unwrap();
            if buf.len() == TELEMETRY_CAPACITY {
                buf.pop_front();
            }
            buf.push_back(TelemetryData {
                datum: telemetry,
                timestamp: Utc::now().naive_utc(),
            });
        }
        Err(e) => log::error!("Failed to get telemetry data => {}", e),
    }
}

pub(crate) fn get_telemetry() -> Result<Option<Telemetry>, WireGuardCommandError> {
    if get_since_timestamp(&LAST_TELEMETRY_QUERY_TS)
        > TELEMETRY_INTERVAL * TELEMETRY_CAPACITY as u64
    {
        *TELEMETRY.write().unwrap() = VecDeque::with_capacity(TELEMETRY_CAPACITY);
    }
    update_timestamp(&LAST_TELEMETRY_QUERY_TS);

    match TELEMETRY.read() {
        Ok(buf) => Ok(Some(Telemetry {
            max_len: TELEMETRY_CAPACITY as u8,
            data: buf.iter().cloned().collect(),
        })),
        Err(e) => Err(WireGuardCommandError::MutexLockFailed(e.to_string())),
    }
}

pub(crate) fn status_tunnel() -> Result<WireGuardStatus, WireGuardCommandError> {
    let wg_status = WG_STATUS
        .read()
        .map_err(|e| WireGuardCommandError::MutexLockFailed(e.to_string()))?;
    Ok(wg_status.clone())
}

fn show_dump(config: &Config) -> Result<BTreeMap<ConnectionId, TelemetryDatum>, WireGuardCommandError> {
    let tunnel_manager = WG_TUNNEL_MANAGER
        .read()
        .map_err(|e| WireGuardCommandError::MutexLockFailed(e.to_string()))?;

    let real_interface = tunnel_manager.real_interface.as_ref().ok_or(WireGuardCommandError::InterfaceMissing)?;

    let output = shell_cmd(&["wg", "show", real_interface, "dump"])?;
    let mut telemetry = BTreeMap::<ConnectionId, TelemetryDatum>::new();

    let dump = String::from_utf8_lossy(&output.stdout);
    for line in dump.trim().lines().skip(1) {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 8 {
            continue;
        }
        let public_key = parts[0];

        for (peer_id, peer_details) in config.network.peers.clone() {
            if wg_quickrs_lib::helpers::wg_public_key_from_private_key(&peer_details.private_key).to_base64() != public_key
            {
                continue;
            }

            let transfer_rx = parts[5].parse::<u64>().unwrap_or(0);
            let transfer_tx = parts[6].parse::<u64>().unwrap_or(0);
            let connection_id =
                wg_quickrs_lib::helpers::get_connection_id(config.network.this_peer, peer_id);

            let (transfer_a_to_b, transfer_b_to_a) = if connection_id.a == config.network.this_peer {
                (transfer_tx, transfer_rx)
            } else {
                (transfer_rx, transfer_tx)
            };

            telemetry.insert(
                connection_id.clone(),
                TelemetryDatum {
                    latest_handshake_at: parts[4].parse::<u64>().unwrap_or(0),
                    transfer_a_to_b,
                    transfer_b_to_a,
                },
            );
            break;
        }
    }
    Ok(telemetry)
}

pub(crate) fn sync_conf(config: &Config) -> Result<(), WireGuardCommandError> {
    let mut tunnel_manager = WG_TUNNEL_MANAGER
        .write()
        .map_err(|e| WireGuardCommandError::MutexLockFailed(e.to_string()))?;

    tunnel_manager.config = Some(config.clone());

    let wg_conf_stripped = get_peer_wg_config(&config.network, &config.network.this_peer, true)
        .map_err(|e| WireGuardCommandError::MutexLockFailed(e.to_string()))?;

    // Debug: Log the generated config to help diagnose sync issues
    log::info!("Generated WireGuard config for sync:\n{}", wg_conf_stripped);

    let mut temp = NamedTempFile::new()
        .map_err(|e| WireGuardCommandError::MutexLockFailed(e.to_string()))?;

    temp.write_all(wg_conf_stripped.as_ref())
        .map_err(|e| WireGuardCommandError::FileWriteError(PathBuf::from(temp.path()), e))?;

    let temp_path = temp.path().to_owned();
    let temp_path_str = temp_path.to_str().unwrap();

    let interface_name = tunnel_manager.real_interface.as_ref().unwrap();
    log::info!("Syncing WireGuard configuration for interface: {}", interface_name);
    
    let sync_result = shell_cmd(&["wg", "syncconf", interface_name, temp_path_str])
        .map_err(|_| WireGuardCommandError::InterfaceSyncFailed());
    
    match sync_result {
        Ok(_) => {
            log::info!("Successfully synced WireGuard configuration for interface: {}", interface_name);
            
            // Restore exit node's 0.0.0.0/0 after sync (since sync_conf filters it out)
            if let Ok(Some(exit_node_id)) = mode::routing_pbr::get_exit_node() {
                if let Some(exit_peer) = config.network.peers.get(&exit_node_id) {
                    let public_key = wg_quickrs_lib::helpers::wg_public_key_from_private_key(&exit_peer.private_key);
                    let public_key_b64 = public_key.to_base64();
                    
                    // Get current allowed IPs for the exit node (excluding 0.0.0.0/0)
                    let mut current_allowed_ips = Vec::new();
                    for (conn_id, conn_details) in &config.network.connections {
                        if conn_id.contains(&exit_node_id) && conn_id.contains(&config.network.this_peer) {
                            let (other_id, allowed_ips) = if conn_id.a == exit_node_id {
                                (&conn_id.b, &conn_details.allowed_ips_a_to_b)
                            } else {
                                (&conn_id.a, &conn_details.allowed_ips_b_to_a)
                            };
                            if other_id == &config.network.this_peer {
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
                        current_allowed_ips.push(format!("{}/32", exit_peer.address));
                    }
                    
                    // Add 0.0.0.0/0 to the list
                    current_allowed_ips.push("0.0.0.0/0".to_string());
                    let allowed_ips_str = current_allowed_ips.join(",");
                    
                    log::info!("Restoring 0.0.0.0/0 to exit node {} after sync", exit_node_id);
                    if let Err(e) = shell_cmd(&["wg", "set", interface_name, "peer", &public_key_b64, 
                                                "allowed-ips", &allowed_ips_str]) {
                        log::warn!("Failed to restore 0.0.0.0/0 to exit node {} after sync: {}", exit_node_id, e);
                    } else {
                        log::info!("Successfully restored 0.0.0.0/0 to exit node {} after sync", exit_node_id);
                    }
                }
            }
            
            Ok(())
        }
        Err(e) => {
            log::error!("Failed to sync WireGuard configuration for interface {}: {}", interface_name, e);
            Err(e)
        }
    }
}

pub(crate) fn disable_tunnel() -> Result<(), WireGuardCommandError> {
    *WG_STATUS
        .write()
        .map_err(|e| WireGuardCommandError::MutexLockFailed(e.to_string()))? =
        WireGuardStatus::UNKNOWN;

    let mut tunnel_manager = WG_TUNNEL_MANAGER
        .write()
        .map_err(|e| WireGuardCommandError::MutexLockFailed(e.to_string()))?;

    tunnel_manager.stop_tunnel()?;
        *WG_STATUS
            .write()
            .map_err(|e| WireGuardCommandError::MutexLockFailed(e.to_string()))? =
            WireGuardStatus::DOWN;

        *TELEMETRY.write().unwrap() = VecDeque::with_capacity(TELEMETRY_CAPACITY);

        Ok(())
}

pub(crate) fn enable_tunnel() -> Result<(), WireGuardCommandError> {
    *WG_STATUS
        .write()
        .map_err(|e| WireGuardCommandError::MutexLockFailed(e.to_string()))? =
        WireGuardStatus::UNKNOWN;

    let mut tunnel_manager = WG_TUNNEL_MANAGER
        .write()
        .map_err(|e| WireGuardCommandError::MutexLockFailed(e.to_string()))?;

    // Try to start the tunnel
    let start_result = tunnel_manager.start_tunnel();
    
    // Check if interface exists (either from successful start or previous run)
    let config = conf::util::get_config().ok();
    let interface_exists = if let Some(ref cfg) = config {
        shell_cmd(&["ip", "link", "show", &cfg.network.name]).is_ok()
    } else {
        false
    };
    
    if start_result.is_ok() {
        *WG_STATUS
            .write()
            .map_err(|e| WireGuardCommandError::MutexLockFailed(e.to_string()))? =
            WireGuardStatus::UP;
    } else if interface_exists {
        // Interface exists even though start failed - might be from previous run
        // Ensure tunnel_manager has the interface name set (interface_exists() should have set it, but ensure it's there)
        if tunnel_manager.real_interface.is_none() {
            if let Some(ref cfg) = config {
                tunnel_manager.real_interface = Some(cfg.network.name.clone());
            }
        }
        
        // Sync the configuration to ensure it matches the current config
        // This is critical to prevent stale configuration from persisting
        if let Some(ref cfg) = config {
            tunnel_manager.config = Some(cfg.clone());
            drop(tunnel_manager); // Release the lock before calling sync_conf
            if let Err(e) = sync_conf(cfg) {
                log::warn!("Failed to sync WireGuard configuration for existing interface: {}. Interface may have stale configuration.", e);
            } else {
                log::info!("Synced WireGuard configuration for existing interface");
            }
        } else {
            log::warn!("Cannot sync WireGuard configuration: config not available");
        }
        // Set status to UP so the system knows the interface is available
    *WG_STATUS
        .write()
        .map_err(|e| WireGuardCommandError::MutexLockFailed(e.to_string()))? =
        WireGuardStatus::UP;
        log::warn!("Tunnel start reported error, but interface exists. Continuing...");
    } else {
        // Start failed and interface doesn't exist - return the error
        return start_result.map_err(|e| WireGuardCommandError::TunnelError(e));
    }
    
    // After the interface is up (or exists), restore peer routes if we're in Router Mode
    // Give the interface a moment to fully initialize
    std::thread::sleep(std::time::Duration::from_millis(500));
    
    // Only restore peer routes if we're in Router Mode
    // Check persisted state to determine if we should restore
    if let Ok(Some(state)) = mode::persist::load_mode_state() {
        if state.last_mode == mode::mode::SystemMode::Router {
            if let Err(e) = mode::mode::restore_peer_routes_after_interface_up() {
                // Log warning but don't fail - the tunnel is up, routes can be restored later
                log::warn!("Failed to restore peer routes after interface creation: {}. Routes may need manual restoration.", e);
            }
            
            // Restore exit node's 0.0.0.0/0 if exit node exists
            if let Some(ref cfg) = config {
                let interface_name = &cfg.network.name;
                if let Ok(Some(exit_node_id)) = mode::routing_pbr::get_exit_node() {
                    if let Some(exit_peer) = cfg.network.peers.get(&exit_node_id) {
                        let public_key = wg_quickrs_lib::helpers::wg_public_key_from_private_key(&exit_peer.private_key);
                        let public_key_b64 = public_key.to_base64();
                        
                        // Get current allowed IPs for the exit node (excluding 0.0.0.0/0)
                        let mut current_allowed_ips = Vec::new();
                        for (conn_id, conn_details) in &cfg.network.connections {
                            if conn_id.contains(&exit_node_id) && conn_id.contains(&cfg.network.this_peer) {
                                let (other_id, allowed_ips) = if conn_id.a == exit_node_id {
                                    (&conn_id.b, &conn_details.allowed_ips_a_to_b)
                                } else {
                                    (&conn_id.a, &conn_details.allowed_ips_b_to_a)
                                };
                                if other_id == &cfg.network.this_peer {
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
                            current_allowed_ips.push(format!("{}/32", exit_peer.address));
                        }
                        
                        // Add 0.0.0.0/0 to the list
                        current_allowed_ips.push("0.0.0.0/0".to_string());
                        let allowed_ips_str = current_allowed_ips.join(",");
                        
                        log::info!("Restoring 0.0.0.0/0 to exit node {} on startup", exit_node_id);
                        if let Err(e) = crate::helpers::shell_cmd(&["wg", "set", interface_name, "peer", &public_key_b64, 
                                                                    "allowed-ips", &allowed_ips_str]) {
                            log::warn!("Failed to restore 0.0.0.0/0 to exit node {} on startup: {}", exit_node_id, e);
                        } else {
                            log::info!("Successfully restored 0.0.0.0/0 to exit node {} on startup", exit_node_id);
                        }
                    }
                }
            }
        } else {
            log::debug!("Not in Router Mode. Skipping peer route restoration.");
        }
    } else {
        log::debug!("No persisted state found. Skipping peer route restoration.");
    }
    
    // Return success if interface exists, even if start_tunnel had issues
    if interface_exists {
    Ok(())
    } else {
        start_result.map_err(|e| WireGuardCommandError::TunnelError(e))
    }
}
