# Router Mode

Router Mode transforms your wg-quickrs host into a VPN gateway, enabling advanced routing capabilities for your WireGuard network.

## Overview

When Router Mode is enabled, your wg-quickrs host can:

1. **Route traffic through an exit node** - Send all internet traffic from connected peers through a remote WireGuard peer
2. **Control LAN access per peer** - Selectively allow or deny peers access to your local network
3. **Monitor connection health** - Real-time metrics for latency, packet loss, and jitter

## Enabling Router Mode

### Via Web UI

1. Navigate to the **Control Center** card
2. Toggle **Router Mode** to ON
3. Enter your LAN subnet(s) in CIDR notation (e.g., `192.168.1.0/24`)
   - Multiple subnets can be specified as comma-separated values: `192.168.1.0/24, 10.0.0.0/8`
4. Click **Enable**

### Requirements

- Linux with `iptables` installed
- IP forwarding enabled (`net.ipv4.ip_forward=1`)
- `NET_ADMIN` capability (if running in Docker)

## Exit Node Selection

Once Router Mode is enabled, the **Gateway Status** card appears with:

- **Exit Node Selector** - Dropdown to choose which peer to route traffic through
- **Health Metrics** - Real-time monitoring of the selected exit node:
  - Up Since timestamp
  - Current endpoint address
  - Latency (ping time)
  - Packet Loss percentage
  - Jitter (latency variance)

### Eligible Exit Nodes

Only peers that advertise a default route (`0.0.0.0/0`) in their `AllowedIPs` are shown as available exit nodes.

## LAN Access Control

Control which peers can access your local network:

### Per-Peer Toggle

In the **Control Center** → **Connected Peers** section, each peer has a **Home** icon button:
- 🏠 **Green** - Peer has LAN access
- 🏠 **Red/Crossed** - Peer is denied LAN access

### How It Works

When LAN access is denied for a peer:
- The peer can still access the internet through the exit node
- The peer cannot reach devices on your local LAN subnet(s)
- Traffic to LAN destinations is blocked via `ip rule` policies

### Default Behavior

- New peers default to having LAN access enabled
- Settings persist across peer reconnections and service restarts

## Multiple LAN Subnets

You can configure multiple LAN subnets for access control:

1. In **System Health & Info** card, click the **LAN Subnet** field
2. Enter comma-separated CIDR values: `192.168.1.0/24, 192.168.2.0/24, 10.0.0.0/8`
3. Press Enter or click away to save

Each subnet will have separate routing rules applied.

## Technical Details

### Policy-Based Routing

Router Mode uses Linux Policy-Based Routing (PBR) with the following rules:

```
# Example ip rules created
ip rule add from <peer_subnet> to <lan_cidr> lookup main priority 19899  # LAN exception
ip rule add from <peer_subnet> lookup <peer_table> priority 20000        # Route to exit node
```

### Firewall Rules

The following iptables rules are managed automatically:

```bash
# NAT for outbound traffic
iptables -t nat -A POSTROUTING -s <wg_subnet> -o <gateway_if> -j MASQUERADE

# Forward rules
iptables -A FORWARD -i <wg_if> -o <gateway_if> -j ACCEPT
iptables -A FORWARD -i <gateway_if> -o <wg_if> -m state --state RELATED,ESTABLISHED -j ACCEPT
```

### State Persistence

Router Mode state is persisted to `router_mode_state.json` in the wg-quickrs config folder:

```json
{
  "mode": "router",
  "lan_cidr": "192.168.1.0/24",
  "peer_lan_access": {
    "peer-uuid-1": true,
    "peer-uuid-2": false
  }
}
```

## Troubleshooting

### Traffic not routing through exit node

1. Verify the exit node peer has `0.0.0.0/0` in AllowedIPs
2. Check that IP forwarding is enabled: `sysctl net.ipv4.ip_forward`
3. Verify iptables rules: `iptables -t nat -L POSTROUTING`

### LAN access not working

1. Verify the LAN CIDR is correct
2. Check ip rules: `ip rule list`
3. Ensure the LAN interface is detected correctly

### Health metrics showing offline

1. Verify the exit node is reachable: `ping <exit_node_ip>`
2. Check WireGuard handshake: `wg show`
3. Verify the peer's endpoint is accessible

