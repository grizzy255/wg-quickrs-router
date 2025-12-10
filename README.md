# wg-quickrs (Gateway Edition)

> 🔀 **Smart WireGuard Routing:** A management tool designed to bypass CGNAT and master Policy-Based Routing.

[![Latest Release](https://img.shields.io/github/v/release/grizzy255/wg-quickrs-router?label=Latest%20Release)](https://github.com/grizzy255/wg-quickrs-router/releases)
[![License](https://img.shields.io/badge/license-GPL--3.0-blue.svg)](LICENSE.txt)

**wg-quickrs Gateway** Transforms a standard Linux host into an intelligent VPN rendezvous point. It solves the headache of connecting to peers behind cellular/ISP firewalls (CGNAT) while maintaining granular control over your LAN traffic.

---

## 🚀 Features

* **⚡ Smart Gateway:** Auto-failover to healthy gateways (3s detection) and auto-failback when stability returns.
* **🌉 CGNAT Traversal:** "Dial out" from restricted networks (LTE/Starlink) to this gateway to establish bi-directional connectivity.
* **🎯 Policy-Based Routing (PBR):** Assign specific LAN devices (e.g., Apple TV) to specific remote exit nodes.
* **🛡️ Per-Peer Isolation:** Each peer gets a dedicated routing table—no more route conflicts.
* **📊 Visual Dashboard:** Real-time health metrics (jitter/latency), topology graphs, and one-click controls.

---

## 💡 The Challenge

Standard WireGuard is great, but it struggles in complex "Road Warrior" or Site-to-Site scenarios involving **CGNAT** (Carrier-Grade NAT) with Policy Based Routing.


1.  **Client Mode VPNs** fail because the remote site (e.g., a cellular modem) has no public IP to accept connections. So you cant do PBR. 
2.  **Server Mode VPNs** allow connection, but lack the granular routing logic (PBR) to decide *which* LAN traffic goes to *which* peer.

### The Solution

**wg-quickrs Gateway** acts as a central "Rendezvous Point." Remote peers connect *out* to the gateway, and the gateway intelligently routes LAN traffic back through them.

```mermaid
flowchart TD
    subgraph LAN ["🏠 Your Local Network"]
        iPhone[iPhone / PC]
        ATV[Apple TV]
        Gateway[<b>wg-quickrs Gateway</b><br/>(This Tool)]
    end

    subgraph Internet ["☁️ Internet"]
        Remote1[<b>Exit Node Peer 1</b><br/>(Remote Site / CGNAT)]
        Remote2[<b>Exit Node Peer 2</b><br/>(Home / VPS)]
    end

    %% Connections
    iPhone -->|Default Route| Gateway
    ATV -->|Policy Route| Gateway
    
    Gateway <==>|WireGuard Tunnel| Remote1
    Gateway <==>|WireGuard Tunnel| Remote2
    
    Remote1 -.->|Public IP A| World[World Wide Web]
    Remote2 -.->|Public IP B| World

    %% Styling
    classDef box fill:#f9f9f9,stroke:#333,stroke-width:2px;
    class Gateway box