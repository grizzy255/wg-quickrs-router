<template>

  <!-- Init Wizard (shown when config doesn't exist) -->
  <init-wizard v-if="showInitWizard"
               :api="api"
               @complete="handleInitComplete">
  </init-wizard>

  <div v-else class="flex flex-col font-mono h-screen transition-colors duration-300">
    <div class="flex-1 flex flex-col bg-page">
    
    <!-- Header -->
    <div class="bg-header border-b border-divider px-6 py-4">
      <div class="container mx-auto max-w-7xl flex items-center justify-between">
        <div class="flex items-center gap-3">
          <h1 class="text-2xl font-bold text-primary">wg-quickrs</h1>
          <span class="px-2 py-1 text-xs font-medium text-secondary bg-button rounded-full">Web Console</span>
        </div>
        <div class="flex items-center gap-3">
          <!-- Dark Mode Toggle -->
          <button
              @click="toggleDarkMode"
              class="h-8 w-8 rounded-md bg-button text-text-button hover:bg-button-hover flex items-center justify-center transition-colors"
              :title="isDarkMode ? 'Switch to Light Mode' : 'Switch to Dark Mode'">
            <Sun v-if="isDarkMode" :size="20" />
            <Moon v-else :size="20" />
          </button>
          <!-- Settings Button -->
          <div class="relative">
            <button
                :class="[settingsDropdownOpen ? 'bg-button-hover': '']"
                class="h-8 w-8 rounded-md bg-button text-text-button hover:bg-button-hover flex items-center justify-center transition-colors"
                @click="settingsDropdownOpen = !settingsDropdownOpen">
              <Settings :size="20" />
            </button>
            <!-- Settings Dropdown -->
            <div v-if="settingsDropdownOpen"
                 class="absolute right-0 top-10 w-24 bg-dropdown border border-divider rounded-md shadow-lg z-20">
              <button
                  class="block w-full text-left px-3 py-2 text-sm text-text-button hover:bg-button rounded-md flex items-center"
                  @click="settingsDropdownOpen = false; logout();">
                <LogOut :size="16" class="mr-2" />
                <span>Logout</span>
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Main Content -->
    <div class="flex-1 overflow-y-auto">
      <div class="container mx-auto max-w-7xl px-6 py-6 text-primary">
        
        <!-- Top Row: Three Cards -->
        <div class="grid grid-cols-1 md:grid-cols-3 gap-6 mb-6">
          
          <!-- System Health & Information Card -->
          <system-health-card
              :web-server-status="webServerStatus"
              :wireguard-status="wireguardStatus"
              :network="network"
              :stringify-endpoint="stringify_endpoint"
              :router-mode="routerMode"
              :lan-cidr="routerModeLanCidr"
              @update-lan-cidr="handleUpdateLanCidr">
          </system-health-card>

          <!-- Control Center Card -->
          <control-center-card
              :wireguard-status="wireguardStatus"
              :router-mode="routerMode"
              :connected-peers="connectedPeers"
              :get-peer-name="getPeerName"
              :get-peer-last-handshake="getPeerLastHandshake"
              :format-last-handshake="formatLastHandshake"
              :has-lan-access="hasLanAccess"
              :lan-access-loading="lanAccessLoading"
              :peer-control-loading="peerControlLoading"
              @toggle-wireguard="dialogId = 'network-toggle'"
              @toggle-router-mode="handleRouterModeToggle"
              @toggle-lan-access="toggleLanAccess"
              @peer-control="handlePeerControl">
          </control-center-card>

          <!-- Gateway Status Card -->
          <div class="bg-card rounded-lg shadow-sm border border-divider p-5 hover:shadow-md transition-shadow duration-200">
            <h2 class="text-lg font-semibold text-primary mb-4 flex items-center gap-2">
              <Router :size="20" class="text-icon" />
              Gateway Status
            </h2>
            <router-mode
                :mode="routerMode"
                :lan-cidr="routerModeLanCidr"
                :api="api"
                :network="network"
                @toggle="handleRouterModeToggle"
                @update:lan-cidr="handleRouterModeLanCidrUpdate">
            </router-mode>
          </div>
        </div>

        <!-- Traffic Analysis Card (Full Width) -->
        <traffic-analysis-card
            :network="network"
            :telemetry="telemetry">
        </traffic-analysis-card>

        <!-- Network Topology Card -->
        <div class="bg-card rounded-lg shadow-sm border border-divider p-5">
          <div class="flex items-center justify-between mb-4">
            <h2 class="text-lg font-semibold text-primary flex items-center gap-2">
              <Network :size="20" class="text-secondary" />
              Network Topology
            </h2>
            <button :disabled="webServerStatus !== 'up'"
                    class="px-4 py-2 bg-green-500 text-white text-sm font-medium rounded-md hover:bg-green-600 disabled:bg-gray-400 disabled:text-gray-600 disabled:cursor-not-allowed transition flex items-center gap-2"
                    @click="dialogId = 'create-peer'">
              <Plus :size="16" />
              Add Peer
            </button>
          </div>
          <div id="graph-app" class="h-[500px] overflow-hidden">
            <map-visual :network="network"
                        :telemetry="telemetry"
                        @peer-selected="onPeerSelected"></map-visual>
          </div>
        </div>

      </div>
    </div>

    <!-- Footer -->
    <footer class="bg-header border-t border-divider px-6 py-3 text-center text-secondary">
      <small>
        <a class="hover:underline" href="https://www.wireguard.com/" target="_blank">
          "WireGuard" and the "WireGuard" logo are registered trademarks of Jason A. Donenfeld.
        </a>
      </small>
      <br/>
      <small>
        <span>© 2025</span>
        <strong>
          <a class="hover:underline" href="https://github.com/grizzy255/wg-quickrs-router" target="_blank">wg-quickrs-router</a>
        </strong>
        <span class="mx-1">•</span>
        <span>forked from</span>
        <a class="hover:underline" href="https://github.com/GodOfKebab/wg-quickrs" target="_blank">GodOfKebab/wg-quickrs</a>
      </small>
    </footer>

    </div><!-- End of dark mode wrapper -->

    <!-- Dialog: Ask Password -->
    <password-dialog v-if="api.does_need_auth"
                     :api="api"></password-dialog>


    <custom-dialog v-if="dialogId === 'network-toggle'" :left-button-click="() => { dialogId = '' }"
                   modal-classes="max-w-xl"
                   :left-button-text="'Cancel'"
                   :right-button-color="wireguardStatus === 'up' ? 'red' : 'green'"
                   :right-button-click="() => { toggleWireGuardNetworking(); dialogId = ''; }"
                   :right-button-text="wireguardStatus === 'up' ? 'Disable' : 'Enable'"
                   class="z-10"
                   icon="danger">
      <h3 class="text-lg leading-6 font-medium text-primary">
        {{ wireguardStatus === 'up' ? 'Disable' : 'Enable' }} the WireGuard Network
      </h3>
      <div class="mt-2 text-sm text-secondary">
        Are you sure you want to {{ wireguardStatus === 'up' ? 'disable' : 'enable' }} the WireGuard
        network?
      </div>
    </custom-dialog>

    <!-- Dialog: Peer View/Edit -->
    <peer-config-dialog v-if="dialogId.startsWith('selected-peer-id=')"
                        v-model:dialog-id="dialogId"
                        :api="api"
                        :network="network"
                        :peer-id="dialogId.slice(17, dialogId.length)"></peer-config-dialog>

    <!-- Dialog: Peer Create -->
    <peer-create-dialog v-if="dialogId === 'create-peer'"
                        v-model:dialog-id="dialogId"
                        :api="api"
                        :network="network"></peer-create-dialog>

    <!-- Dialog: Router Mode -->
    <router-mode-dialog
        v-if="dialogId === 'router-mode'"
        :router-mode-lan-cidr="routerModeLanCidr"
        @confirm="handleRouterModeConfirm"
        @cancel="dialogId = ''">
    </router-mode-dialog>

    <!-- Dialog: Router Mode Error -->
    <custom-dialog v-if="dialogId === 'router-mode-error'"
                   :left-button-click="() => { dialogId = ''; routerModeError = '' }"
                   modal-classes="max-w-xl"
                   :left-button-text="'OK'"
                   :right-button-click="() => { dialogId = ''; routerModeError = '' }"
                   :right-button-text="''"
                   class="z-10"
                   icon="danger">
      <h3 class="text-lg leading-6 font-medium text-primary">
        Cannot Change Mode
      </h3>
      <div class="mt-2 text-sm text-secondary">
        <div v-if="routerModeError">{{ routerModeError }}</div>
        <div v-else>Mode cannot be changed while peers are configured. To change the mode, you must first delete all peers from the network. After deleting all peers, you can switch between Host and Router modes.</div>
      </div>
    </custom-dialog>

  </div>
</template>

<script>
import API from "@/src/js/api.js";
import MapVisual from "@/src/components/map-visual.vue";
import CustomDialog from "@/src/components/dialogs/custom-dialog.vue";
import PasswordDialog from "@/src/components/dialogs/password-dialog.vue";
import PeerConfigDialog from "@/src/components/dialogs/peer-config-dialog.vue";
import PeerCreateDialog from "@/src/components/dialogs/peer-create-dialog.vue";
import RouterMode from "@/src/components/router-mode.vue";
import RouterModeDialog from "@/src/components/dialogs/router-mode-dialog.vue";
import InitWizard from "@/src/components/init-wizard.vue";
import SystemHealthCard from "@/src/components/cards/SystemHealthCard.vue";
import ControlCenterCard from "@/src/components/cards/ControlCenterCard.vue";
import TrafficAnalysisCard from "@/src/components/cards/TrafficAnalysisCard.vue";
import { Settings, LogOut, Router, Network, Plus, Sun, Moon } from 'lucide-vue-next';

import dayjs from 'dayjs';
import relativeTime from 'dayjs/plugin/relativeTime';
import init from '@/pkg/wg_quickrs_lib.js';
import WireGuardHelper from "@/src/js/wg-helper.js";


dayjs.extend(relativeTime);

export default {
  name: "app",
  components: {
    PasswordDialog,
    MapVisual,
    CustomDialog,
    PeerConfigDialog,
    PeerCreateDialog,
    RouterMode,
    RouterModeDialog,
    InitWizard,
    SystemHealthCard,
    ControlCenterCard,
    TrafficAnalysisCard,
    Settings,
    LogOut,
    Router,
    Network,
    Plus,
    Sun,
    Moon
  },
  data() {
    return {
      refreshRate: 1000,
      webServerStatus: 'unknown',
      wireguardStatus: 'unknown',
      ServerStatusEnum: {
        'unknown': 0,
        'down': 1,
        'up': 2
      },
      dialogId: '',
      network: {},
      telemetry: null,
      digest: '',
      last_fetch: {
        rfc3339: "",
        readable: "",
        since: -1,
      },
      wasmInitialized: false,
      api: {does_need_auth: false},
      settingsDropdownOpen: false,
      routerMode: 'unknown', // 'unknown', 'host', 'router'
      routerModeLanCidr: null,
      routerModeError: '',
      showInitWizard: false,
      isInitialized: false,  // Default to false - will be set by checkInitStatus()
      isDarkMode: false,     // Dark mode state
      peerLanAccess: {},     // Track LAN access per peer: { peerId: boolean }
      lanAccessLoading: {},  // Track LAN access toggle loading: { peerId: boolean }
      peerControlLoading: {} // Track peer control loading: { peerId: 'reconnect'|'stop'|'start' }
    }
  },
  async mounted() {
    // Load dark mode preference from localStorage
    const savedDarkMode = localStorage.getItem('darkMode');
    if (savedDarkMode !== null) {
      this.isDarkMode = savedDarkMode === 'true';
    } else {
      // Check system preference
      this.isDarkMode = window.matchMedia('(prefers-color-scheme: dark)').matches;
    }
    // Apply dark class to <html> element for Tailwind
    if (this.isDarkMode) {
      document.documentElement.classList.add('dark');
    } else {
      document.documentElement.classList.remove('dark');
    }

    if (!this.wasmInitialized) {
      try {
        await init();
        this.wasmInitialized = true;
      } catch (err) {
        console.error('WASM failed to load:', err);
      }
    }

    this.api = new API();
    if (localStorage.getItem('remember') === 'true') {
      this.api.token = localStorage.getItem('token') || '';
    }

    // Check if initialization is needed
    await this.checkInitStatus();

    // Only set up refresh interval if initialized
    if (this.isInitialized && !this.showInitWizard) {
      setInterval(() => {
        this.refresh()
      }, this.refreshRate)
    }
  },
  computed: {
    connectedPeers() {
      if (!this.telemetry || !this.telemetry.data || this.telemetry.data.length === 0 || !this.network || !this.network.peers) {
        return [];
      }
      
      const now = Math.floor(Date.now() / 1000);
      const HANDSHAKE_TIMEOUT = 180; // 3 minutes - consider peer connected if handshake within last 3 minutes
      const connectedPeerIds = new Set();
      
      // Get the latest telemetry data
      const latestData = this.telemetry.data[this.telemetry.data.length - 1];
      if (!latestData || !latestData.datum) {
        return [];
      }
      
      // Extract peer IDs from connection IDs and check handshake times
      for (const [connectionId, telemetryDatum] of Object.entries(latestData.datum)) {
        if (telemetryDatum.latest_handshake_at && (now - telemetryDatum.latest_handshake_at) < HANDSHAKE_TIMEOUT) {
          // Connection ID format: "uuid1*uuid2" - extract both peer IDs
          const parts = connectionId.split('*');
          if (parts.length === 2) {
            const peer1 = parts[0];
            const peer2 = parts[1];
            // Only include peers that exist in network and are not this peer
            if (this.network.peers[peer1] && peer1 !== this.network.this_peer) {
              connectedPeerIds.add(peer1);
            }
            if (this.network.peers[peer2] && peer2 !== this.network.this_peer) {
              connectedPeerIds.add(peer2);
            }
          }
        }
      }
      
      return Array.from(connectedPeerIds);
    }
  },
  methods: {
    stringify_endpoint(endpoint) {
      return WireGuardHelper.stringify_endpoint(endpoint);
    },
    getPeerName(peerId) {
      if (!this.network || !this.network.peers || !this.network.peers[peerId]) {
        return peerId.substring(0, 8) + '...';
      }
      return this.network.peers[peerId].name || peerId.substring(0, 8) + '...';
    },
    getPeerLastHandshake(peerId) {
      if (!this.telemetry || !this.telemetry.data || this.telemetry.data.length === 0) {
        return null;
      }
      
      const latestData = this.telemetry.data[this.telemetry.data.length - 1];
      if (!latestData || !latestData.datum) {
        return null;
      }
      
      // Find the connection that includes this peer
      for (const [connectionId, telemetryDatum] of Object.entries(latestData.datum)) {
        const parts = connectionId.split('*');
        if (parts.length === 2 && (parts[0] === peerId || parts[1] === peerId)) {
          return telemetryDatum.latest_handshake_at || null;
        }
      }
      
      return null;
    },
    formatLastHandshake(timestamp) {
      if (!timestamp) return 'Never';
      const now = Math.floor(Date.now() / 1000);
      const diff = now - timestamp;
      if (diff < 60) return `${diff}s ago`;
      if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
      if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
      return `${Math.floor(diff / 86400)}d ago`;
    },
    async refresh() {
      // Don't refresh if not initialized - double check to prevent API calls
      if (!this.isInitialized || this.showInitWizard) {
        return;
      }

      this.last_fetch.since = this.last_fetch.rfc3339 ? new Date() - new Date(this.last_fetch.rfc3339) : -1;

      // Fetch router mode status (this method also checks isInitialized)
      this.fetchRouterMode();

      let need_to_update_network = true;
      if (this.digest.length === 64) {
        await this.api.get_network_summary('?only_digest=true').then(summary => {
          this.webServerStatus = 'up';
          this.wireguardStatus = summary.status;
          need_to_update_network = this.digest !== summary.digest;
          this.telemetry = summary.telemetry;

          this.last_fetch.rfc3339 = summary.timestamp;
          const last_fetch_date = (new Date(Date.parse(this.last_fetch.rfc3339)))
          this.last_fetch.readable = `${last_fetch_date} [${dayjs(last_fetch_date).fromNow()}]`;
          this.last_fetch.since = 0;
        }).catch(err => {
          this.telemetry = null;
          this.wireguardStatus = 'unknown';
          if (err.toString() === 'TypeError: Load failed') {
            this.webServerStatus = 'down';
          } else {
            this.webServerStatus = 'unknown';
          }
        });
      }

      if (need_to_update_network) {
        await this.api.get_network_summary('?only_digest=false').then(summary => {
          this.webServerStatus = 'up';
          this.digest = summary.digest;
          this.telemetry = summary.telemetry;
          this.network = summary.network;
          this.network.static_peer_ids = [];
          this.network.roaming_peer_ids = [];
          Object.entries(summary.network.peers).forEach(([peerId, peerDetails]) => {
            if (peerDetails.endpoint.enabled) {
              this.network.static_peer_ids.push(peerId);
            } else {
              this.network.roaming_peer_ids.push(peerId);
            }
          })
          this.wireguardStatus = summary.status

          this.last_fetch.rfc3339 = summary.timestamp;
          const last_fetch_date = (new Date(Date.parse(this.last_fetch.rfc3339)))
          this.last_fetch.readable = `${last_fetch_date} [${dayjs(last_fetch_date).fromNow()}]`;
          this.last_fetch.since = 0;
        }).catch(err => {
          this.telemetry = null;
          this.wireguardStatus = 'unknown';
          if (err.toString() === 'TypeError: Load failed') {
            this.webServerStatus = 'down';
          } else {
            this.webServerStatus = 'unknown';
          }
        });
      }
    },
    toggleWireGuardNetworking() {
      // Don't toggle if not initialized
      if (!this.isInitialized) {
        return;
      }
      const curr = this.wireguardStatus === 'up';
      this.api.post_wireguard_status({status: curr ? 'down' : 'up'})
          .then(() => {
            this.refresh();
          }).catch(() => {});
      this.wireguardStatus = 'unknown';
    },
    onPeerSelected(peer_id) {
      this.dialogId = `selected-peer-id=${peer_id}`;
    },
    logout() {
      this.api.token = '';
      localStorage.removeItem('token');
      localStorage.removeItem('remember');
      // Don't refresh if not initialized
      if (this.isInitialized) {
        this.refresh();
      }
    },
    async checkInitStatus() {
      try {
        const status = await this.api.get_init_status();
        this.isInitialized = status.initialized;
        this.showInitWizard = !status.initialized;
      } catch (err) {
        // If API call fails, assume not initialized
        console.error('Failed to check init status:', err);
        this.isInitialized = false;
        this.showInitWizard = true;
      }
    },
    async fetchRouterMode() {
      // Don't fetch router mode if not initialized (endpoint doesn't exist in init mode)
      if (!this.isInitialized) {
        this.routerMode = 'unknown';
        return;
      }
      try {
        const modeData = await this.api.get_mode();
        this.routerMode = modeData.mode || 'host';
        this.routerModeLanCidr = modeData.lan_cidr || null;
        
        // Fetch LAN access status when in router mode
        if (this.routerMode === 'router') {
          this.fetchLanAccessStatus();
        }
      } catch {
        this.routerMode = 'unknown';
      }
    },
    async fetchLanAccessStatus() {
      try {
        const result = await this.api.get_peer_lan_access();
        if (result && result.peer_lan_access) {
          const lanAccess = {};
          for (const [peerId, info] of Object.entries(result.peer_lan_access)) {
            lanAccess[peerId] = info.has_lan_access;
          }
          this.peerLanAccess = lanAccess;
        }
      } catch (error) {
        console.error('Failed to load LAN access status:', error);
      }
    },
    hasLanAccess(peerId) {
      // Default to true if not specified
      return this.peerLanAccess[peerId] !== false;
    },
    async toggleLanAccess(peerId) {
      if (this.lanAccessLoading[peerId]) return;
      
      const currentAccess = this.hasLanAccess(peerId);
      const newAccess = !currentAccess;
      
      this.lanAccessLoading = { ...this.lanAccessLoading, [peerId]: true };
      
      try {
        await this.api.set_peer_lan_access(peerId, newAccess);
        this.peerLanAccess = { ...this.peerLanAccess, [peerId]: newAccess };
      } catch (error) {
        console.error('Failed to toggle LAN access:', error);
        alert(`Failed to toggle LAN access: ${error.message}`);
      } finally {
        this.lanAccessLoading = { ...this.lanAccessLoading, [peerId]: false };
      }
    },
    async handlePeerControl(peerId, action) {
      if (this.peerControlLoading[peerId]) return;
      
      this.peerControlLoading = { ...this.peerControlLoading, [peerId]: action };
      
      try {
        await this.api.peer_control(peerId, action);
      } catch (error) {
        console.error(`Failed to ${action} peer:`, error);
        alert(`Failed to ${action} peer: ${error.message}`);
      } finally {
        this.peerControlLoading = { ...this.peerControlLoading, [peerId]: null };
      }
    },
    handleInitComplete() {
      this.showInitWizard = false;
      // Don't set isInitialized to true yet - wait for service to restart
      // Poll for init status until service restarts and detects the config
      this.pollUntilInitialized();
    },
    async pollUntilInitialized() {
      const maxAttempts = 30; // 30 attempts = 30 seconds
      let attempts = 0;
      
      const poll = async () => {
        try {
          const status = await this.api.get_init_status();
          if (status.initialized) {
            // Try to call a config-dependent endpoint to verify service is out of init mode
            try {
              await this.api.get_network_summary('?only_digest=true');
              // If we get here, service is out of init mode
              this.isInitialized = true;
              // Set up refresh interval
              setInterval(() => {
                this.refresh()
              }, this.refreshRate);
              // Reload the page to start fresh with the new config
              window.location.reload();
              return;
            } catch (err) {
              // Endpoint not available yet - service still in init mode
              // Continue polling
            }
          }
          
          attempts++;
          if (attempts < maxAttempts) {
            setTimeout(poll, 1000); // Poll every second
          } else {
            // Timeout - show message that service needs restart
            alert('Initialization complete! Please restart the wg-quickrs service for changes to take effect.\n\nRun: systemctl restart wg-quickrs.service');
            this.showInitWizard = false;
            this.isInitialized = false;
          }
        } catch (err) {
          attempts++;
          if (attempts < maxAttempts) {
            setTimeout(poll, 1000);
          } else {
            alert('Initialization complete! Please restart the wg-quickrs service for changes to take effect.\n\nRun: systemctl restart wg-quickrs.service');
            this.showInitWizard = false;
            this.isInitialized = false;
          }
        }
      };
      
      // Start polling after a short delay
      setTimeout(poll, 1000);
    },
    async handleRouterModeToggle() {
      // First check if mode can be switched (applies to both Host and Router Mode)
      try {
        const canSwitch = await this.api.can_switch_mode();
        if (!canSwitch.can_switch) {
          // Show modal with error message
          this.dialogId = 'router-mode-error';
          return;
        }
      } catch {
        this.dialogId = 'router-mode-error';
        return;
      }

      // If switching to Host Mode, do it directly
      if (this.routerMode === 'router') {
        // Switch to Host Mode directly
        try {
          const result = await this.api.toggle_mode('host', null);
          this.routerMode = result.mode || 'host';
          this.routerModeLanCidr = null;
        } catch (err) {
          this.routerModeError = err.message || 'Failed to switch to Host Mode.';
          this.dialogId = 'router-mode-error';
        }
        return;
      }

      // If switching to Router Mode, show the popup dialog
      // Show Router Mode dialog
      this.dialogId = 'router-mode';
    },
    async handleRouterModeConfirm(cidr) {
      // Switch to Router Mode with the provided CIDR
      try {
        const result = await this.api.toggle_mode('router', cidr);
        this.routerMode = result.mode || 'router';
        this.routerModeLanCidr = result.lan_cidr || cidr;
        this.routerModeError = '';
        this.dialogId = '';
      } catch (err) {
        this.routerModeError = err.message || 'Failed to switch to Router Mode.';
        // Show error modal
        this.dialogId = 'router-mode-error';
      }
    },
    toggleDarkMode() {
      this.isDarkMode = !this.isDarkMode;
      localStorage.setItem('darkMode', this.isDarkMode.toString());
      // Apply dark class to <html> element for Tailwind
      if (this.isDarkMode) {
        document.documentElement.classList.add('dark');
      } else {
        document.documentElement.classList.remove('dark');
      }
    },
    async handleUpdateLanCidr(cidr) {
      // Update LAN CIDR while staying in router mode
      try {
        const result = await this.api.toggle_mode('router', cidr);
        this.routerModeLanCidr = result.lan_cidr || cidr;
      } catch (err) {
        console.error('Failed to update LAN CIDR:', err);
        alert(`Failed to update LAN CIDR: ${err.message}`);
      }
    },
    handleRouterModeLanCidrUpdate(cidr) {
      this.routerModeLanCidr = cidr;
    }
  }
}
</script>

