# wg-quickrs

> **🆕 What's New**
> 
> - **Router Mode** - Transform your host into a VPN gateway with exit node selection
> - **Per-Peer LAN Access Control** - Allow/deny individual peers access to your local network
> - **Multiple LAN Subnet Support** - Configure multiple comma-separated CIDRs for LAN access rules
> - **Real-time Health Monitoring** - Live latency, packet loss, and jitter metrics for exit nodes
> - **Web-based Initialization Wizard** - Configure everything from the browser on first run
> - **Enhanced Traffic Analysis** - Improved graphs with tooltips and timeline markers
> - **Redesigned Dashboard** - Three-card layout: System Health, Control Center, and Gateway Status

---

[![License](https://img.shields.io/github/license/godofkebab/wg-quickrs?logo=GitHub&color=brightgreen)](https://github.com/GodOfKebab/wg-quickrs)
![Static Badge](https://img.shields.io/badge/amd64%20%7C%20arm64%20%7C%20arm%2Fv7%20%20-%20grey?label=arch)
![Static Badge](https://img.shields.io/badge/Linux%20%7C%20macOS%20%20-%20black?label=platform)

[![Release](https://img.shields.io/github/v/tag/godofkebab/wg-quickrs?logo=github&label=latest%20tag&color=blue)](https://github.com/godofkebab/wg-quickrs/releases/latest)
[![Docker](https://img.shields.io/docker/image-size/godofkebab/wg-quickrs?logo=docker&color=%232496ED)](https://hub.docker.com/repository/docker/godofkebab/wg-quickrs)
[![Docker](https://img.shields.io/docker/pulls/godofkebab/wg-quickrs?logo=docker&color=%232496ED)](https://hub.docker.com/repository/docker/godofkebab/wg-quickrs/tags)
![Dynamic TOML Badge](https://img.shields.io/badge/dynamic/toml?url=https%3A%2F%2Fraw.githubusercontent.com%2FGodOfKebab%2Fwg-quickrs%2Frefs%2Fheads%2Fmain%2Fsrc%2Fwg-quickrs%2FCargo.toml&query=package.rust-version&logo=rust&label=rust&color=%23000000)
![Dynamic JSON Badge](https://img.shields.io/badge/dynamic/json?url=https%3A%2F%2Fraw.githubusercontent.com%2FGodOfKebab%2Fwg-quickrs%2Frefs%2Fheads%2Fmain%2Fsrc%2Fwg-quickrs-web%2Fpackage.json&query=dependencies.vue&logo=vue.js&label=vue&color=%234FC08D)

✨ An intuitive multi-peer `wg` wrapper written in 🦀 Rust (`wg-quick` alternative).

⚡ Rust + Vue + WASM + WireGuard = 🧪 one [static binary](docs/notes/static-binary.md) + 📝 one [YAML file](docs/notes/schema.md) to rule them all 🪄

Run it on your [router](docs/quick-start/router.md), [server](docs/quick-start/server.md), or [docker host](docs/quick-start/docker.md) and manage your WireGuard VPN from a [terminal](docs/quick-start/cli.md) or a web interface.

<p align="center">
  <img src="https://yasar.idikut.cc/project-assets/wg-quickrs-speedtest.gif" alt="speedtest demo">
</p>

<p align="center">
  <img src="https://yasar.idikut.cc/project-assets/wg-quickrs-demo.gif" alt="usage demo">
</p>

Features:
- Interactive graph to configure your P2P network
- HTTPS support and password login with JWT-based API authentication
- Automatic firewall/NAT setup (`iptables` for Debian/Linux or `pf` for macOS, both usually come preinstalled with the OS)
- **Router Mode** with exit node selection - route all traffic through a remote peer
- **Per-peer LAN access control** - selectively allow/deny peers access to your local network
- **Real-time traffic analysis** with bandwidth graphs
- **Web-based initialization wizard** - configure everything from the browser
- If you are not feeling like dealing with VPN/networking on your machine, you can also just use the CLI or the web console to create `.conf` files/QR codes for your network peers.

---

## Quick Start

To get started, see quick start guides for [routers](docs/quick-start/router.md), [servers](docs/quick-start/server.md), or [docker hosts](docs/quick-start/docker.md).

## Router Mode

Router Mode transforms your wg-quickrs host into a VPN gateway, allowing connected peers to route their internet traffic through a remote exit node.

**Features:**
- **Exit Node Selection** - Choose which peer to route traffic through
- **LAN Access Control** - Per-peer toggle to allow/deny access to your local network (supports multiple LAN subnets)
- **Health Monitoring** - Real-time latency, packet loss, and jitter metrics for the exit node
- **Automatic Routing** - Policy-based routing (PBR) with automatic iptables/firewall rule management

See [Router Mode Guide](docs/notes/router-mode.md) for detailed configuration.
