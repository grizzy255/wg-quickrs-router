# wg-quickrs (Gateway Edition)

> 🔀 A WireGuard management tool with Router Mode for CGNAT/cellular peers

**Forked from [GodOfKebab/wg-quickrs](https://github.com/GodOfKebab/wg-quickrs)**

---

## 🆕 What's New in This Fork

| Feature | Description |
|---------|-------------|
| **Router Mode** | Transform your host into a VPN gateway with exit node selection |
| **Per-Peer LAN Access** | Toggle LAN access on/off for individual peers |
| **Multi-CIDR Support** | Configure multiple LAN subnets (comma-separated) |
| **Health Monitoring** | Real-time latency, packet loss, and jitter metrics |
| **Web Init Wizard** | Browser-based first-time setup |
| **Redesigned Dashboard** | Three-card layout: System Health, Control Center, Gateway Status |

---

## 🎯 Problem Statement

We faced an infrastructure challenge where remote peers were behind **CGNAT** (Carrier-Grade NAT) or cellular networks (LTE/Starlink), making them unreachable from the outside.

### Why Standard Solutions Failed

| Approach | Problem |
|----------|---------|
| **Client Mode** | Gateway can't dial peers — they have no public IP |
| **Server Mode (UniFi, etc.)** | Peers can connect, but no granular Policy-Based Routing to control where traffic goes |

**We needed:** A solution that accepts incoming connections from hidden peers while providing advanced routing logic.

---

## 🛠️ The Solution

**wg-quickrs Gateway Edition** acts as a central intelligent rendezvous point.

Deploy on a Linux host with a public IP (or port forwarding) to:

1. **Bypass CGNAT** — Peers initiate outbound connections to this server
2. **Granular PBR** — Define per-peer routing rules (exit nodes, backup servers, etc.)
3. **LAN Bridging** — Automatic iptables masquerading to bridge peers into internal subnets
4. **Access Control** — Allow or deny LAN access per peer

```
┌─────────────────┐     ┌──────────────────────┐     ┌─────────────────┐
│  Peer behind    │────▶│   wg-quickrs         │────▶│  Exit Node /    │
│  CGNAT/LTE      │     │   Gateway            │     │  LAN Resources  │
└─────────────────┘     │                      │     └─────────────────┘
                        │  • Router Mode       │
┌─────────────────┐     │  • PBR Engine        │     ┌─────────────────┐
│  Starlink Peer  │────▶│  • LAN Access Ctrl   │────▶│  Backup Server  │
└─────────────────┘     │  • Health Monitor    │     └─────────────────┘
                        └──────────────────────┘
```

---

## ✨ Features

### Core WireGuard Management
- **Multi-peer support** — Manage unlimited peers from one interface
- **Interactive network graph** — Visual P2P network topology
- **QR codes & .conf export** — Easy peer provisioning
- **HTTPS & JWT auth** — Secure web access with password login

### Router Mode (This Fork)
- **Exit node selection** — Route all peer traffic through a selected peer
- **Per-peer LAN access** — Toggle home icon to allow/deny LAN access
- **Multiple LAN subnets** — Comma-separated CIDRs (e.g., `192.168.1.0/24, 10.0.0.0/8`)
- **Persistent settings** — LAN access survives peer reconnects

### Monitoring & Dashboard
- **Real-time health metrics** — Latency, packet loss, jitter
- **Traffic graphs** — Enhanced with tooltips and grid lines
- **Three-card layout:**
  - System Health & Info (status, tunnel IP, LAN subnets)
  - Control Center (toggles, connected peers with controls)
  - Gateway Status (exit node health, uptime, endpoint)

---

## 🚀 Quick Start

### Docker (Recommended)

**Step 1: Initialize**
```bash
docker compose -f docker-compose.init.yml up
```
Access http://your-server:8080 and complete the web wizard.

**Step 2: Run the Agent**
```bash
docker compose -f docker-compose.agent.yml up -d
```

### Manual Installation

```bash
# Download and run installer
curl -fsSL https://raw.githubusercontent.com/grizzy255/wg-quickrs-router/main/installer.sh | bash

# Initialize (web wizard)
wg-quickrs agent init --web-init

# Run the agent
wg-quickrs agent run --config /etc/wireguard/wg-quickrs.yaml
```

---

## 🔧 Router Mode Usage

### Enable Router Mode
1. Open the web dashboard
2. Toggle **Router Mode** in the Control Center card
3. Enter your LAN subnet(s): `192.168.1.0/24` or `192.168.1.0/24, 10.0.0.0/8`

### Select Exit Node
1. In **Gateway Status** card, click dropdown
2. Select an online peer as exit node
3. All peer traffic routes through the selected exit

### Control LAN Access
1. In **Control Center** → Connected Peers
2. Click the 🏠 home icon to toggle LAN access per peer
3. Red = denied, default = allowed

---

## 📁 Configuration

Configuration stored in `/etc/wireguard/wg-quickrs.yaml`:

```yaml
interface:
  name: WireStream
  address: 10.100.105.1/24
  port: 51822
  private_key: <generated>

peers:
  - name: exit-node-1
    public_key: <key>
    endpoint: 1.2.3.4:51820
    allowed_ips: 0.0.0.0/0
    
  - name: mobile-peer
    public_key: <key>
    # No endpoint - peer dials in (CGNAT)
    allowed_ips: 10.100.105.2/32
```

Router Mode state persisted in `/var/lib/wg-quickrs/router_mode_state.json`.

---

## 🔒 Security Notes

- Web interface protected by password + JWT tokens
- HTTPS support available (see docs)
- LAN access denied peers can still reach other WireGuard peers
- Firewall rules managed automatically via iptables

---

## 📚 Documentation

- [Docker Setup](docs/quick-start/docker.md)
- [Server Installation](docs/quick-start/server.md)
- [Router Mode Details](docs/notes/router-mode.md)

---

## 🙏 Credits

- Original project: [GodOfKebab/wg-quickrs](https://github.com/GodOfKebab/wg-quickrs)
- Built with Rust, Vue.js, and WireGuard

---

## 📄 License

GPL-3.0 — See [LICENSE.txt](LICENSE.txt)
