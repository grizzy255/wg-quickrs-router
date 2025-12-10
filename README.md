# wg-quickrs (Gateway Edition)

> 🔀 **Smart WireGuard Routing:** A management tool designed to bypass CGNAT and master Policy-Based Routing.

[![Latest Release](https://img.shields.io/github/v/release/grizzy255/wg-quickrs-router?label=Latest%20Release)](https://github.com/grizzy255/wg-quickrs-router/releases)
[![License](https://img.shields.io/badge/license-GPL--3.0-blue.svg)](LICENSE.txt)

**wg-quickrs Gateway** transforms a standard Linux host into an intelligent VPN rendezvous point. It solves the headache of connecting to peers behind cellular/ISP firewalls (CGNAT) while maintaining granular control over your LAN traffic.

---

## 🚀 Features

* **⚡ Smart Gateway:** Auto-failover to healthy gateways (3s detection) and auto-failback when stability returns.
* **🌉 CGNAT Traversal:** "Dial out" from restricted networks (LTE/Starlink) to this gateway to establish bi-directional connectivity.
* **🎯 Policy-Based Routing (PBR):** Assign specific LAN devices (e.g., Apple TV) to specific remote exit nodes.
* **🛡️ Per-Peer Isolation:** Each peer gets a dedicated routing table—no more route conflicts.
* **📊 Visual Dashboard:** Real-time health metrics (jitter/latency), topology graphs, and one-click controls.

---

## 💡 The Challenge

Standard WireGuard is great, but it struggles in complex "Road Warrior" or Site-to-Site scenarios involving **CGNAT** (Carrier-Grade NAT).

1. **Client Mode VPNs** fail because the remote site (e.g., a cellular modem) has no public IP to accept connections.
2. **Server Mode VPNs** allow connection, but lack the granular routing logic (PBR) to decide *which* LAN traffic goes to *which* peer.

### The Solution

**wg-quickrs Gateway** acts as a central "Rendezvous Point." Remote peers connect *out* to the gateway, and the gateway intelligently routes LAN traffic back through them.

```mermaid
flowchart LR
    %% --- Styling Definitions ---
    classDef base font-family:sans-serif,font-size:14px
    
    %% Client Nodes (Blue Theme)
    classDef client fill:#e1f5fe,stroke:#0288d1,stroke-width:2px,color:#01579b,rx:10,ry:10
    
    %% Gateway Node (Teal/Green Theme - Central Hub)
    classDef gateway fill:#e0f2f1,stroke:#00897b,stroke-width:3px,color:#004d40,rx:10,ry:10,shadow:5px
    
    %% Exit Nodes (Orange Theme - External)
    classDef exit fill:#fff3e0,stroke:#f57c00,stroke-width:2px,stroke-dasharray: 5 5,color:#e65100,rx:5,ry:5
    
    %% Internet (Purple Theme)
    classDef internet fill:#f3e5f5,stroke:#8e24aa,stroke-width:2px,color:#4a148c
    
    %% Subgraph Styling
    classDef subgraphStyle fill:#ffffff,stroke:#cfd8dc,stroke-width:1px,color:#546e7a,rx:5,ry:5

    %% --- Graph Content ---
    
    subgraph Local ["🏠 Local Network"]
        direction TB
        ATV["📺 Apple TV"]:::client
    end

    subgraph Roaming ["📱 Roaming Devices"]
        direction TB
        iPhone["📱 iPhone / Laptop"]:::client
    end

    subgraph GW_Zone ["⚡ Gateway Infrastructure"]
        direction TB
        GW["<b>Main Gateway</b><br/>Router Mode<br/>Health Checks<br/>Routing Logic"]:::gateway
    end

    subgraph Exits ["☁️ Remote Exit Nodes"]
        direction LR
        Peer1["Peer 1<br/>(CGNAT Site)"]:::exit
        Peer2["Peer 2<br/>(Home / VPS)"]:::exit
    end

    World(("🌐 Internet")):::internet

    %% --- Connections ---
    
    %% Client Traffic
    ATV --> GW
    iPhone --> GW
    
    %% Encrypted Tunnels
    GW -- "WireGuard Tunnel" <-- Peer1
    GW -- "WireGuard Tunnel" <-- Peer2
    
    %% Internet Access
    Peer1 -. "NAT Egress" .-> World
    Peer2 -. "NAT Egress" .-> World

    %% Apply Styles
    class Local,Roaming,GW_Zone,Exits subgraphStyle
    
    %% Link Styling for smoother look
    linkStyle default stroke:#546e7a,stroke-width:2px,fill:none
```

**Traffic Flow Example:**
- Apple TV ➔ Gateway ➔ Exit Peer 1 ➔ Internet *(Appears as Peer 1's IP)*
- iPhone ➔ Gateway ➔ Exit Peer 2 ➔ Internet *(Appears as Peer 2's IP)*

---

## 📸 Dashboard

![Dashboard](docs/figures/dashboard-dark1.png)

---

## 📦 Quick Start

### 1. Installation

The easiest way to run the gateway is using the pre-compiled binary.

```bash
# 1. Download the latest binary
sudo curl -L -o /usr/local/bin/wg-quickrs https://github.com/grizzy255/wg-quickrs-router/releases/latest/download/wg-quickrs-linux-amd64
sudo chmod +x /usr/local/bin/wg-quickrs

# 2. Install dependencies
sudo apt install wireguard-tools iptables
```

### 2. Service Setup

Create a persistent systemd service.

```bash
sudo tee /etc/systemd/system/wg-quickrs.service > /dev/null << 'EOF'
[Unit]
Description=wg-quickrs WireGuard Gateway
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/wg-quickrs agent run --config /etc/wireguard/wg-quickrs.yaml
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable --now wg-quickrs
```

### 3. Access

Open your browser to `http://<your-server-ip>:80`.

> **Note:** Configure your initial admin credentials via the web-based setup wizard on first launch.

---

## 🆕 What's New in v2.0

| Feature | Description |
|---------|-------------|
| **Smart Gateway** | Detects connection drops (3 consecutive ping failures) and automatically reroutes traffic |
| **Auto-Failback** | Restores preferred route after 60 seconds of stability |
| **Reduced False Positives** | Improved health monitoring algorithms |

---

## 📁 Configuration

| File | Purpose |
|------|---------|
| `/etc/wireguard/wg-quickrs.yaml` | Main configuration file |
| `/etc/wireguard/router_mode_state.json` | Router Mode persistent state |

---

## 🤝 Contributing & Credits

- **Maintainer:** [grizzy255](https://github.com/grizzy255)
- **Original Project:** [GodOfKebab/wg-quickrs](https://github.com/GodOfKebab/wg-quickrs)

> 🤖 **Development Note:** This project utilizes AI-assisted development (Claude/Cursor). While functional and tested, code contributions to improve idiomatic Rust patterns are highly welcome!

---

## 📄 License

GPL-3.0 — See [LICENSE.txt](LICENSE.txt)

---

<sub>"WireGuard" and the "WireGuard" logo are registered trademarks of Jason A. Donenfeld.</sub>
