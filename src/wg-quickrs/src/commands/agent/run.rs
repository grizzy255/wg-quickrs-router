use crate::web::server;
use crate::{conf, wireguard, mode};
use thiserror::Error;
use tokio::try_join;
use crate::conf::util::ConfUtilError;

#[derive(Error, Debug)]
pub enum AgentRunError {
    #[error("configuration error: {0}")]
    Conf(#[from] ConfUtilError),
    #[error("io error: {0}")]
    IO(#[from] std::io::Error),
}

pub async fn run_agent() -> Result<(), AgentRunError> {
    // Try to get config, but allow web server to start even if config doesn't exist
    // (so users can access the initialization wizard)
    let config_result = conf::util::get_config();
    let config = match config_result {
        Ok(c) => Some(c),
        Err(conf::util::ConfUtilError::Read(_, _)) => {
            // Config file doesn't exist - start web server in "init mode"
            log::warn!("Config file not found. Starting web server in initialization mode.");
            None
        }
        Err(e) => {
            // Other config errors - fail
            return Err(AgentRunError::Conf(e));
        }
    };
    
    // Restore Router Mode on startup if it was active before restart
    // This must happen before starting the web server to ensure routing is correct
    // We add a small delay to allow network services (DHCP, etc.) to finish initializing
    // Note: We call initialize_mode_on_startup() even when config is None, because
    // it needs to check if config file exists and validate persisted state
    if config.is_some() {
        log::info!("Config loaded. Waiting for network services to stabilize...");
        // Give network services time to add their routes
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
    
    log::info!("Checking for Router Mode restoration...");
    if let Err(e) = mode::mode::initialize_mode_on_startup() {
        // Log error but don't fail - allow the service to start even if restoration fails
        // The user can manually fix the routing if needed
        log::warn!("Failed to restore Router Mode on startup: {}. Service will continue, but routing may need manual intervention.", e);
    }
    
    if let Some(ref cfg) = config {
        // Start health monitor as background task
        tokio::spawn(async {
            if let Err(e) = mode::routing_pbr::start_health_monitor().await {
                log::error!("Health monitor error: {}", e);
            }
        });
        
        let web_future = server::run_web_server(cfg);
        let vpn_future = wireguard::cmd::run_vpn_server(cfg);
    try_join!(web_future, vpn_future)?;
    } else {
        // Start web server without config (init mode)
        let web_future = server::run_web_server_init_mode();
        web_future.await?;
    }
    Ok(())
}
