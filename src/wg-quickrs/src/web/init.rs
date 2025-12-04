// Web-based initialization endpoints
use crate::WG_QUICKRS_CONFIG_FILE;
use crate::commands::agent::init::initialize_agent;
use crate::commands::agent::init::get_interfaces;
use crate::commands::agent::init::recommend_interface;
use actix_web::{HttpRequest, HttpResponse, Responder, web};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use wg_quickrs_cli::agent::InitOptions;

#[derive(Serialize, Deserialize)]
pub struct InitStatusResponse {
    pub initialized: bool,
    pub config_path: String,
}

#[derive(Serialize, Deserialize)]
pub struct InitData {
    // Network settings
    pub network_name: Option<String>,
    pub network_subnet: Option<String>,
    
    // Agent web settings
    pub agent_web_address: Option<String>,
    pub agent_web_http_enabled: Option<bool>,
    pub agent_web_http_port: Option<u16>,
    pub agent_web_https_enabled: Option<bool>,
    pub agent_web_https_port: Option<u16>,
    pub agent_web_https_tls_cert: Option<String>,
    pub agent_web_https_tls_key: Option<String>,
    pub agent_web_password_enabled: Option<bool>,
    pub agent_web_password: Option<String>,
    
    // Agent VPN settings
    pub agent_vpn_enabled: Option<bool>,
    pub agent_vpn_port: Option<u16>,
    
    // Agent firewall settings
    pub agent_firewall_enabled: Option<bool>,
    pub agent_firewall_utility: Option<String>,
    pub agent_firewall_gateway: Option<String>,
    
    // Agent peer settings
    pub agent_peer_name: Option<String>,
    pub agent_peer_vpn_internal_address: Option<String>,
    pub agent_peer_vpn_endpoint: Option<String>,
    pub agent_peer_kind: Option<String>,
    pub agent_peer_icon_enabled: Option<bool>,
    pub agent_peer_icon_src: Option<String>,
    pub agent_peer_dns_enabled: Option<bool>,
    pub agent_peer_dns_addresses: Option<Vec<String>>,
    pub agent_peer_mtu_enabled: Option<bool>,
    pub agent_peer_mtu_value: Option<u16>,
    
    // Default peer settings
    pub default_peer_kind: Option<String>,
    pub default_peer_icon_enabled: Option<bool>,
    pub default_peer_icon_src: Option<String>,
    pub default_peer_dns_enabled: Option<bool>,
    pub default_peer_dns_addresses: Option<Vec<String>>,
    pub default_peer_mtu_enabled: Option<bool>,
    pub default_peer_mtu_value: Option<u16>,
    
    // Default connection settings
    pub default_connection_persistent_keepalive_enabled: Option<bool>,
    pub default_connection_persistent_keepalive_period: Option<u16>,
}

#[derive(Serialize, Deserialize)]
pub struct InterfaceInfo {
    pub name: String,
    pub ip: String,
    pub recommended: bool,
}

#[derive(Serialize)]
pub struct InitInfoResponse {
    pub interfaces: Vec<InterfaceInfo>,
    pub recommended_interface: Option<InterfaceInfo>,
}

// Check if initialization is needed
pub async fn get_init_status(_req: HttpRequest) -> impl Responder {
    let config_file = WG_QUICKRS_CONFIG_FILE.get().unwrap();
    let initialized = config_file.exists();
    
    HttpResponse::Ok().json(InitStatusResponse {
        initialized,
        config_path: config_file.display().to_string(),
    })
}

// Get initialization info (interfaces, defaults, etc.)
pub async fn get_init_info(_req: HttpRequest) -> impl Responder {
    let interfaces = get_interfaces();
    let recommended = recommend_interface();
    
    let interface_info: Vec<InterfaceInfo> = interfaces
        .iter()
        .map(|iface| {
            let ip_str = match iface.ip() {
                IpAddr::V4(v4) => v4.to_string(),
                _ => String::new(),
            };
            let is_recommended = recommended.as_ref().map(|r| r.name == iface.name).unwrap_or(false);
            
            InterfaceInfo {
                name: iface.name.clone(),
                ip: ip_str,
                recommended: is_recommended,
            }
        })
        .collect();
    
    let recommended_info = recommended.map(|r| {
        let ip_str = match r.ip() {
            IpAddr::V4(v4) => v4.to_string(),
            _ => String::new(),
        };
        InterfaceInfo {
            name: r.name.clone(),
            ip: ip_str,
            recommended: true,
        }
    });
    
    HttpResponse::Ok().json(InitInfoResponse {
        interfaces: interface_info,
        recommended_interface: recommended_info,
    })
}

// Submit initialization data
pub async fn post_init(_req: HttpRequest, body: web::Bytes) -> impl Responder {
    let body_str = String::from_utf8_lossy(&body);
    let init_data: InitData = match serde_json::from_str(&body_str) {
        Ok(data) => data,
        Err(e) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": format!("Invalid JSON: {}", e)
            }));
        }
    };
    
    // Convert InitData to InitOptions
    use std::path::PathBuf;
    use ipnet::Ipv4Net;
    use std::net::Ipv4Addr;
    
    let init_opts = InitOptions {
        no_prompt: Some(true), // We're providing all values via web
        network_name: init_data.network_name,
        network_subnet: init_data.network_subnet.and_then(|s| s.parse::<Ipv4Net>().ok()),
        agent_web_address: init_data.agent_web_address.and_then(|s| s.parse::<Ipv4Addr>().ok()),
        agent_web_http_enabled: init_data.agent_web_http_enabled,
        agent_web_http_port: init_data.agent_web_http_port,
        agent_web_https_enabled: init_data.agent_web_https_enabled,
        agent_web_https_port: init_data.agent_web_https_port,
        agent_web_https_tls_cert: init_data.agent_web_https_tls_cert.map(|s| PathBuf::from(s)),
        agent_web_https_tls_key: init_data.agent_web_https_tls_key.map(|s| PathBuf::from(s)),
        agent_web_password_enabled: init_data.agent_web_password_enabled,
        agent_web_password: init_data.agent_web_password,
        agent_vpn_enabled: init_data.agent_vpn_enabled,
        agent_vpn_port: init_data.agent_vpn_port,
        agent_firewall_enabled: init_data.agent_firewall_enabled,
        agent_firewall_utility: init_data.agent_firewall_utility.map(|s| PathBuf::from(s)),
        agent_firewall_gateway: init_data.agent_firewall_gateway,
        agent_peer_name: init_data.agent_peer_name,
        agent_peer_vpn_internal_address: init_data.agent_peer_vpn_internal_address.and_then(|s| s.parse::<Ipv4Addr>().ok()),
        agent_peer_vpn_endpoint: init_data.agent_peer_vpn_endpoint,
        agent_peer_kind: init_data.agent_peer_kind,
        agent_peer_icon_enabled: init_data.agent_peer_icon_enabled,
        agent_peer_icon_src: init_data.agent_peer_icon_src,
        agent_peer_dns_enabled: init_data.agent_peer_dns_enabled,
        agent_peer_dns_addresses: init_data.agent_peer_dns_addresses.map(|addrs| {
            addrs.iter().filter_map(|s| s.parse::<Ipv4Addr>().ok()).collect()
        }).unwrap_or_default(),
        agent_peer_mtu_enabled: init_data.agent_peer_mtu_enabled,
        agent_peer_mtu_value: init_data.agent_peer_mtu_value,
        default_peer_kind: init_data.default_peer_kind,
        default_peer_icon_enabled: init_data.default_peer_icon_enabled,
        default_peer_icon_src: init_data.default_peer_icon_src,
        default_peer_dns_enabled: init_data.default_peer_dns_enabled,
        default_peer_dns_addresses: init_data.default_peer_dns_addresses.map(|addrs| {
            addrs.iter().filter_map(|s| s.parse::<Ipv4Addr>().ok()).collect()
        }).unwrap_or_default(),
        default_peer_mtu_enabled: init_data.default_peer_mtu_enabled,
        default_peer_mtu_value: init_data.default_peer_mtu_value,
        default_connection_persistent_keepalive_enabled: init_data.default_connection_persistent_keepalive_enabled,
        default_connection_persistent_keepalive_period: init_data.default_connection_persistent_keepalive_period,
        // Script fields must be Some(bool) when no_prompt is true, not None
        agent_peer_script_pre_up_enabled: Some(false),
        agent_peer_script_pre_up_line: vec![],
        agent_peer_script_post_up_enabled: Some(false),
        agent_peer_script_post_up_line: vec![],
        agent_peer_script_pre_down_enabled: Some(false),
        agent_peer_script_pre_down_line: vec![],
        agent_peer_script_post_down_enabled: Some(false),
        agent_peer_script_post_down_line: vec![],
        default_peer_script_pre_up_enabled: Some(false),
        default_peer_script_pre_up_line: vec![],
        default_peer_script_post_up_enabled: Some(false),
        default_peer_script_post_up_line: vec![],
        default_peer_script_pre_down_enabled: Some(false),
        default_peer_script_pre_down_line: vec![],
        default_peer_script_post_down_enabled: Some(false),
        default_peer_script_post_down_line: vec![],
    };
    
    // Call the initialization function
    match initialize_agent(&init_opts) {
        Ok(_) => {
            HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "message": "Initialization completed successfully"
            }))
        }
        Err(e) => {
            HttpResponse::BadRequest().json(serde_json::json!({
                "error": e.to_string()
            }))
        }
    }
}

