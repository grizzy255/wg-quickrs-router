// All persistent state across restarts:
// last mode, LAN CIDR, peer table mapping, prefix active/backup state
//
// Responsibilities:
// - STEP 2: Persist mode state (restart logic)
// - STEP 7: Persist peer table mappings and prefix active/backup state

use super::mode::SystemMode;
use crate::WG_QUICKRS_CONFIG_FOLDER;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Mutex;
use thiserror::Error;

const MODE_STATE_FILE: &str = "router_mode_state.json";
const MODE_STATE_TEMP_FILE: &str = "router_mode_state.json.tmp";

// Global mutex to prevent concurrent state file operations
// This prevents race conditions where multiple threads try to save/load simultaneously
static STATE_FILE_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

#[derive(Error, Debug)]
pub enum PersistenceError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Deserialization error: {0}")]
    DeserializationError(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModeState {
    pub last_mode: SystemMode,
    pub lan_cidr: Option<String>,
    pub peer_table_ids: HashMap<String, u32>, // peer_id -> table_id
    pub prefix_active_backup: HashMap<String, PrefixState>, // prefix -> state
    #[serde(default)]
    pub peer_first_handshake: HashMap<String, u64>, // peer_id -> first_handshake_timestamp (Unix seconds)
    #[serde(default)]
    pub peer_last_online_state: HashMap<String, bool>, // peer_id -> was_online (tracks previous online state)
    #[serde(default)]
    pub peer_last_successful_ping: HashMap<String, u64>, // peer_id -> last_successful_ping_timestamp (Unix seconds)
    #[serde(default = "default_peer_lan_access")]
    pub peer_lan_access: HashMap<String, bool>, // peer_id -> has_lan_access (default true)
    #[serde(default)]
    pub auto_failover: bool, // Smart Gateway - automatically switch to healthy peer when exit node goes offline
    #[serde(default)]
    pub primary_exit_node: Option<String>, // User's preferred gateway - for fail-back after failover
    #[serde(default)]
    pub primary_online_since: Option<u64>, // Timestamp when primary came back online (for fail-back timing)
}

fn default_peer_lan_access() -> HashMap<String, bool> {
    HashMap::new() // Empty means all peers default to having LAN access
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PrefixState {
    pub active_peer_id: String,
    pub backup_peer_ids: Vec<String>,
}

fn get_state_file_path() -> Result<PathBuf, PersistenceError> {
    let config_folder = WG_QUICKRS_CONFIG_FOLDER
        .get()
        .ok_or_else(|| PersistenceError::IoError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Config folder not initialized"
        )))?;
    
    Ok(config_folder.join(MODE_STATE_FILE))
}

// Save mode state to file using atomic writes
// This prevents file corruption from concurrent access or interrupted writes
pub fn save_mode_state(state: &ModeState) -> Result<(), PersistenceError> {
    // Acquire lock to prevent concurrent state file operations
    let _lock = STATE_FILE_LOCK.lock().map_err(|e| {
        PersistenceError::IoError(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to acquire state file lock: {}", e)
        ))
    })?;
    
    let file_path = get_state_file_path()?;
    let temp_path = file_path.with_file_name(MODE_STATE_TEMP_FILE);
    
    // Ensure config folder exists
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| PersistenceError::IoError(e))?;
    }
    
    // Serialize to JSON
    let json = serde_json::to_string_pretty(state)
        .map_err(|e| PersistenceError::SerializationError(e.to_string()))?;
    
    // ATOMIC WRITE: Write to temp file first
    {
        let mut file = File::create(&temp_path)
            .map_err(|e| PersistenceError::IoError(e))?;
        
        file.write_all(json.as_bytes())
            .map_err(|e| PersistenceError::IoError(e))?;
        
        // Ensure data is flushed to disk before renaming
        file.sync_all()
            .map_err(|e| PersistenceError::IoError(e))?;
    }
    
    // ATOMIC RENAME: Replace the original file with the temp file
    // This is atomic on most filesystems (ext4, etc.)
    fs::rename(&temp_path, &file_path)
        .map_err(|e| {
            // If rename fails, try to clean up temp file
            let _ = fs::remove_file(&temp_path);
            PersistenceError::IoError(e)
        })?;
    
    log::debug!("Saved router mode state to {:?} (atomic write)", file_path);
    Ok(())
}

// Load mode state from file
// Self-healing: if file is empty or corrupted, delete it and return None
pub fn load_mode_state() -> Result<Option<ModeState>, PersistenceError> {
    // Acquire lock to prevent concurrent state file operations
    let _lock = STATE_FILE_LOCK.lock().map_err(|e| {
        PersistenceError::IoError(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to acquire state file lock: {}", e)
        ))
    })?;
    
    let file_path = get_state_file_path()?;
    let temp_path = file_path.with_file_name(MODE_STATE_TEMP_FILE);
    
    // Clean up any leftover temp file from interrupted writes
    if temp_path.exists() {
        log::debug!("Cleaning up leftover temp state file");
        let _ = fs::remove_file(&temp_path);
    }
    
    // Check if file exists
    if !file_path.exists() {
        return Ok(None);
    }
    
    // Read file
    let mut file = File::open(&file_path)
        .map_err(|e| PersistenceError::IoError(e))?;
    
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .map_err(|e| PersistenceError::IoError(e))?;
    
    // Check for empty file
    if contents.trim().is_empty() {
        log::warn!("Router mode state file is empty. Deleting corrupted file for self-recovery.");
        if let Err(e) = fs::remove_file(&file_path) {
            log::warn!("Failed to delete empty state file: {}", e);
        }
        return Ok(None);
    }
    
    // Deserialize from JSON - with self-healing on corruption
    match serde_json::from_str::<ModeState>(&contents) {
        Ok(state) => {
            log::debug!("Loaded router mode state from {:?}", file_path);
            Ok(Some(state))
        }
        Err(e) => {
            log::warn!("Router mode state file is corrupted ({}). Deleting for self-recovery.", e);
            if let Err(del_err) = fs::remove_file(&file_path) {
                log::warn!("Failed to delete corrupted state file: {}", del_err);
            }
            Ok(None)
        }
    }
}

// Clear mode state (when switching to Host Mode)
pub fn clear_mode_state() -> Result<(), PersistenceError> {
    // Acquire lock to prevent concurrent state file operations
    let _lock = STATE_FILE_LOCK.lock().map_err(|e| {
        PersistenceError::IoError(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to acquire state file lock: {}", e)
        ))
    })?;
    
    let file_path = get_state_file_path()?;
    let temp_path = file_path.with_file_name(MODE_STATE_TEMP_FILE);
    
    // Delete the state file if it exists
    if file_path.exists() {
        fs::remove_file(&file_path)
            .map_err(|e| PersistenceError::IoError(e))?;
        log::info!("Cleared router mode state file {:?}", file_path);
    }
    
    // Also clean up any temp file
    if temp_path.exists() {
        let _ = fs::remove_file(&temp_path);
    }
    
    Ok(())
}

/// Validate persisted state against current config and clean up orphaned entries
/// Returns true if state is valid (has matching peers), false if it's a fresh start
pub fn validate_and_cleanup_persisted_state(
    state: &mut ModeState,
    current_peer_ids: &HashSet<String>,
) -> bool {
    // Collect peer IDs from persisted state
    let persisted_peer_ids: HashSet<String> = state.peer_table_ids.keys().cloned().collect();
    
    // If persisted state has no peer routing tables yet, that's OK - it just means
    // Router Mode was enabled but no exit nodes were configured yet
    if persisted_peer_ids.is_empty() {
        log::info!("Persisted state has no peer routing tables yet. This is valid for newly enabled Router Mode.");
        return true;
    }
    
    // Find matching peers (peers that exist in both persisted state and current config)
    let matching_peers: HashSet<String> = persisted_peer_ids
        .intersection(current_peer_ids)
        .cloned()
        .collect();
    
    // If no peers match, it's a fresh start (config was completely replaced)
    if matching_peers.is_empty() {
        log::info!("No matching peers found between persisted state and current config. This appears to be a fresh start.");
        return false;
    }
    
    // Clean up orphaned entries (peers that exist in persisted state but not in current config)
    let orphaned_peers: Vec<String> = persisted_peer_ids
        .difference(current_peer_ids)
        .cloned()
        .collect();
    
    if !orphaned_peers.is_empty() {
        log::info!("Found {} orphaned peer(s) in persisted state that don't exist in current config. Cleaning up...", orphaned_peers.len());
        
        for peer_id in &orphaned_peers {
            // Remove from peer_table_ids
            state.peer_table_ids.remove(peer_id);
            
            // Remove from peer health tracking
            state.peer_first_handshake.remove(peer_id);
            state.peer_last_online_state.remove(peer_id);
            state.peer_last_successful_ping.remove(peer_id);
            
            // Remove from peer LAN access settings
            state.peer_lan_access.remove(peer_id);
            
            // Remove from prefix_active_backup if this peer was an exit node
            state.prefix_active_backup.retain(|_prefix, prefix_state| {
                let mut updated = false;
                
                // Remove if this peer was the active peer
                if prefix_state.active_peer_id == *peer_id {
                    updated = true;
                }
                
                // Remove from backup peer list
                prefix_state.backup_peer_ids.retain(|id| id != peer_id);
                
                !updated
            });
            
            log::debug!("Removed orphaned peer {} from persisted state", peer_id);
        }
        
        log::info!("Cleaned up {} orphaned peer(s) from persisted state", orphaned_peers.len());
    }
    
    log::info!("Validated persisted state: {} matching peer(s) found", matching_peers.len());
    true
}

