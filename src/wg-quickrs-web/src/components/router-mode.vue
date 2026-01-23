<template>
  <div class="flex flex-col">
    <!-- Exit Node (shown in Router Mode when peers have default routes) -->
    <div v-if="mode === 'router' && peersWithDefaultRoute.length > 0" class="space-y-2">
      <div class="text-sm text-secondary flex items-center gap-2">
        <Server :size="16" class="text-icon" />
        Exit Node
      </div>
      
      <!-- Single peer: Display read-only -->
      <div v-if="peersWithDefaultRoute.length === 1" class="text-sm text-primary font-medium">
        <div class="flex items-center gap-2">
          <span>{{ getPeerName(peersWithDefaultRoute[0]) }}</span>
          <span v-if="getHealthStatus(peersWithDefaultRoute[0])" 
                :class="getStatusIndicatorClass(peersWithDefaultRoute[0])"
                class="w-2 h-2 rounded-full"
                :title="getStatusTooltip(peersWithDefaultRoute[0])"></span>
          <span class="text-xs text-muted ml-1">(automatic)</span>
        </div>
        <div v-if="getHealthStatus(peersWithDefaultRoute[0])" class="text-xs mt-1 space-y-0.5 pl-6">
          <div class="flex items-center justify-between">
            <span class="text-secondary font-medium">{{ getHealthStatus(peersWithDefaultRoute[0]).is_online ? 'Up Since:' : 'Down Since:' }}</span> 
            <span class="text-primary">
              <span v-if="getHealthStatus(peersWithDefaultRoute[0]).first_handshake">
                {{ formatUpSince(getHealthStatus(peersWithDefaultRoute[0]).first_handshake) }}
              </span>
              <span v-else-if="!getHealthStatus(peersWithDefaultRoute[0]).is_online">
                Unknown (never seen online)
              </span>
              <span v-else>Unknown</span>
            </span>
          </div>
          <div v-if="getHealthStatus(peersWithDefaultRoute[0]).endpoint" class="flex items-center justify-between">
            <span class="text-secondary font-medium">Endpoint:</span>
            <span class="text-primary">{{ getHealthStatus(peersWithDefaultRoute[0]).endpoint }}</span>
          </div>
          <div v-if="getHealthStatus(peersWithDefaultRoute[0]).latency_ms !== null && getHealthStatus(peersWithDefaultRoute[0]).latency_ms !== undefined" class="flex items-center justify-between">
            <span class="text-secondary font-medium">Latency:</span>
            <span class="text-primary">{{ getHealthStatus(peersWithDefaultRoute[0]).latency_ms }}ms</span>
          </div>
          <div v-if="getHealthStatus(peersWithDefaultRoute[0]).packet_loss_percent !== null && getHealthStatus(peersWithDefaultRoute[0]).packet_loss_percent !== undefined" class="flex items-center justify-between">
            <span class="text-secondary font-medium">Packet Loss:</span> 
            <span :class="getHealthStatus(peersWithDefaultRoute[0]).packet_loss_percent > 5 ? 'text-red-600' : getHealthStatus(peersWithDefaultRoute[0]).packet_loss_percent > 1 ? 'text-yellow-600' : 'text-green-600'">
              {{ getHealthStatus(peersWithDefaultRoute[0]).packet_loss_percent.toFixed(2) }}%
            </span>
          </div>
          <div v-if="getHealthStatus(peersWithDefaultRoute[0]).jitter_ms !== null && getHealthStatus(peersWithDefaultRoute[0]).jitter_ms !== undefined" class="flex items-center justify-between">
            <span class="text-secondary font-medium">Jitter:</span>
            <span class="text-primary">{{ getHealthStatus(peersWithDefaultRoute[0]).jitter_ms }}ms</span>
          </div>
        </div>
      </div>
      
      <!-- Multiple peers: Show as selectable buttons -->
      <div v-else class="space-y-3">
        <div class="flex flex-col gap-1.5">
          <div v-for="peerId in peersWithDefaultRoute" 
               :key="peerId"
               :class="[
                 'flex items-center justify-between py-2 px-2 text-sm rounded transition-all',
                 peerId === selectedExitNode && selectedExitNode !== exitNode
                   ? 'bg-badge-info-bg border-l-2 border-blue-500' 
                   : peerId === exitNode
                   ? 'bg-badge-success-bg border-l-2 border-green-500'
                   : 'hover:bg-button border-l-2 border-transparent',
                 loading ? 'opacity-50' : ''
               ]">
            <!-- Clickable area for selection -->
            <div class="flex items-center gap-2 flex-grow cursor-pointer"
                 @click="!loading && (selectedExitNode = peerId)">
              <span class="text-primary font-medium">{{ getPeerName(peerId) }}</span>
              <span v-if="peerId === exitNode"
                    class="px-2 py-0.5 text-xs font-medium rounded-full bg-badge-success-bg text-badge-success-text border border-badge-success-border border-opacity-30">
                Active
              </span>
              <span v-if="getHealthStatus(peerId)" 
                    :class="getStatusBadgeClass(peerId)"
                    class="px-2 py-0.5 text-xs font-medium rounded-full border border-opacity-20">
                {{ getStatusText(peerId) }}
              </span>
            </div>
          </div>
        </div>
        
        <!-- Health status for selected peer -->
        <div v-if="selectedExitNode && getHealthStatus(selectedExitNode)" 
             class="mt-3 pt-3 border-t border-divider">
          <div class="text-xs space-y-0.5 pl-6">
            <div class="flex items-center justify-between">
              <span class="text-secondary font-medium">{{ getHealthStatus(selectedExitNode).is_online ? 'Up Since:' : 'Down Since:' }}</span> 
              <span class="text-primary">
                <span v-if="getHealthStatus(selectedExitNode).first_handshake">
                  {{ formatUpSince(getHealthStatus(selectedExitNode).first_handshake) }}
                </span>
                <span v-else-if="!getHealthStatus(selectedExitNode).is_online">
                  Unknown (never seen online)
                </span>
                <span v-else>Unknown</span>
              </span>
            </div>
            <div v-if="getHealthStatus(selectedExitNode).endpoint" class="flex items-center justify-between">
              <span class="text-secondary font-medium">Endpoint:</span> 
              <span class="text-primary">{{ getHealthStatus(selectedExitNode).endpoint }}</span>
            </div>
            <div v-if="getHealthStatus(selectedExitNode).latency_ms !== null && getHealthStatus(selectedExitNode).latency_ms !== undefined" class="flex items-center justify-between">
              <span class="text-secondary font-medium">Latency:</span> 
              <span class="text-primary">{{ getHealthStatus(selectedExitNode).latency_ms }}ms</span>
            </div>
            <div v-if="getHealthStatus(selectedExitNode).packet_loss_percent !== null && getHealthStatus(selectedExitNode).packet_loss_percent !== undefined" class="flex items-center justify-between">
              <span class="text-secondary font-medium">Packet Loss:</span> 
              <span :class="getHealthStatus(selectedExitNode).packet_loss_percent > 5 ? 'text-red-600' : getHealthStatus(selectedExitNode).packet_loss_percent > 1 ? 'text-yellow-600' : 'text-green-600'">
                {{ getHealthStatus(selectedExitNode).packet_loss_percent.toFixed(2) }}%
              </span>
            </div>
            <div v-if="getHealthStatus(selectedExitNode).jitter_ms !== null && getHealthStatus(selectedExitNode).jitter_ms !== undefined" class="flex items-center justify-between">
              <span class="text-secondary font-medium">Jitter:</span> 
              <span class="text-primary">{{ getHealthStatus(selectedExitNode).jitter_ms }}ms</span>
            </div>
          </div>
        </div>
        
        <button
            v-if="selectedExitNode && selectedExitNode !== exitNode && !loading"
            @click="handleExitNodeApply"
            class="w-full mt-3 px-3 py-2 text-sm bg-blue-600 text-white rounded hover:bg-blue-700 transition focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 font-medium disabled:bg-gray-400 disabled:text-gray-600"
            aria-label="Apply exit node selection">
          Apply
        </button>
        
        <!-- Progress indicator when switching gateway -->
        <div v-if="loading" class="text-sm text-badge-info-text p-3 bg-badge-info-bg rounded border border-badge-info-border">
          <div class="flex items-center gap-2">
            <svg class="animate-spin h-5 w-5 text-blue-600" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
              <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
              <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
            </svg>
            <span>Switching gateway... This may take a few seconds.</span>
          </div>
        </div>
      </div>
    </div>
    <div v-else-if="mode === 'host'" class="text-sm text-muted">
      Host mode active
    </div>
  </div>
</template>

<script>
import { Server } from 'lucide-vue-next';

export default {
  name: "router-mode",
  components: {
    Server
  },
  props: {
    mode: {
      type: String,
      default: 'unknown' // 'unknown', 'host', 'router'
    },
    lanCidr: {
      type: String,
      default: null
    },
    api: {
      type: Object,
      required: true
    },
    network: {
      type: Object,
      default: null
    }
  },
  emits: ['update:mode', 'update:lanCidr', 'toggle', 'update:exitNode', 'update:healthStatus'],
  data() {
    return {
      exitNode: null,
      peersWithDefaultRoute: [],
      selectedExitNode: null,
      loading: false,
      healthStatus: {},
      healthPollInterval: null
    }
  },
  watch: {
    mode(newMode) {
      if (newMode === 'router') {
        this.loadExitNodeInfo();
        this.startHealthPolling();
      } else {
        this.exitNode = null;
        this.peersWithDefaultRoute = [];
        this.selectedExitNode = null;
        this.stopHealthPolling();
      }
    },
    exitNode(newExitNode) {
      // Emit the new exit node to parent component
      this.$emit('update:exitNode', newExitNode);
    },
    healthStatus: {
      handler(newHealthStatus) {
        // Emit health status to parent component for the traffic graph
        this.$emit('update:healthStatus', newHealthStatus);
      },
      deep: true
    }
  },
  mounted() {
    if (this.mode === 'router') {
      this.loadExitNodeInfo();
      this.startHealthPolling();
    }
  },
  beforeUnmount() {
    this.stopHealthPolling();
  },
  methods: {
    handleToggle() {
      this.$emit('toggle');
    },
    startHealthPolling() {
      // Poll every 2 seconds to get updated health status from backend
      // Backend updates every 1 second, so 2 second polling gives fresh data
      this.stopHealthPolling(); // Clear any existing interval
      this.healthPollInterval = setInterval(() => {
        if (this.mode === 'router' && this.network) {
          this.updateHealthStatus();
        }
      }, 2000);
    },
    stopHealthPolling() {
      if (this.healthPollInterval) {
        clearInterval(this.healthPollInterval);
        this.healthPollInterval = null;
      }
    },
    async updateHealthStatus() {
      // Update health status and check for exit node changes (Smart Gateway failover/failback)
      if (!this.network) return;
      
      try {
        const info = await this.api.get_exit_node_info();
        
        // Update health status
        if (info.health_status && Array.isArray(info.health_status)) {
          const healthMap = {};
          info.health_status.forEach(h => {
            healthMap[h.peer_id] = h;
          });
          // Only update if health status actually changed to minimize re-renders
          const healthChanged = JSON.stringify(this.healthStatus) !== JSON.stringify(healthMap);
          if (healthChanged) {
            this.healthStatus = healthMap;
          }
        }
        
        // Update exit node if changed (Smart Gateway failover/failback)
        const newExitNode = info.exit_node || null;
        if (newExitNode !== this.exitNode) {
          this.exitNode = newExitNode;
          this.selectedExitNode = newExitNode;
        }
      } catch (error) {
        console.error('Failed to update health status:', error);
      }
    },
    async loadExitNodeInfo() {
      if (!this.network) return;
      
      this.loading = true;
      try {
        const info = await this.api.get_exit_node_info();
        this.exitNode = info.exit_node || null;
        this.peersWithDefaultRoute = info.peers_with_default_route || [];
        // Set selectedExitNode to current exit node, or first peer if no exit node
        this.selectedExitNode = this.exitNode || (this.peersWithDefaultRoute.length > 0 ? this.peersWithDefaultRoute[0] : null);
        
        // Update health status
        if (info.health_status && Array.isArray(info.health_status)) {
          const healthMap = {};
          info.health_status.forEach(h => {
            healthMap[h.peer_id] = h;
          });
          this.healthStatus = healthMap;
        }
      } catch (error) {
        console.error('Failed to load exit node info:', error);
        this.peersWithDefaultRoute = [];
        this.exitNode = null;
        this.selectedExitNode = null;
        this.healthStatus = {};
      } finally {
        this.loading = false;
      }
    },
    getHealthStatus(peerId) {
      return this.healthStatus[peerId] || null;
    },
    getStatusIndicatorClass(peerId) {
      const health = this.getHealthStatus(peerId);
      if (!health) return 'bg-badge-warning-bg';
      if (health.is_online) {
        // Always green for online peers
        return 'bg-badge-success-bg';
      }
      return 'bg-badge-error-bg';
    },
    getStatusText(peerId) {
      const health = this.getHealthStatus(peerId);
      if (!health) return 'unknown';
      return health.is_online ? 'Online' : 'Offline';
    },
    getStatusBadgeClass(peerId) {
      const health = this.getHealthStatus(peerId);
      if (!health) {
        return 'bg-badge-warning-bg text-badge-warning-text';
      }
      if (health.is_online) {
        // Check if there's high packet loss (degraded state)
        if (health.packet_loss_percent !== null && health.packet_loss_percent !== undefined) {
          if (health.packet_loss_percent > 5) {
            return 'bg-badge-error-bg text-badge-error-text';
          } else if (health.packet_loss_percent > 1) {
            return 'bg-badge-warning-bg text-badge-warning-text';
          }
        }
        return 'bg-badge-success-bg text-badge-success-text';
      }
      return 'bg-badge-error-bg text-badge-error-text';
    },
    getStatusTooltip(peerId) {
      const health = this.getHealthStatus(peerId);
      if (!health) return 'Status unknown';
      let tooltip = health.is_online ? 'Online' : 'Offline';
      if (health.latency_ms !== null && health.latency_ms !== undefined) {
        tooltip += ` - Latency: ${health.latency_ms}ms`;
      }
      if (health.packet_loss_percent !== null && health.packet_loss_percent !== undefined) {
        tooltip += ` - Loss: ${health.packet_loss_percent.toFixed(2)}%`;
      }
      if (health.jitter_ms !== null && health.jitter_ms !== undefined) {
        tooltip += ` - Jitter: ${health.jitter_ms}ms`;
      }
      if (health.last_handshake) {
        tooltip += ` - Last: ${this.formatLastHandshake(health.last_handshake)}`;
      }
      return tooltip;
    },
    getHealthStatusText(peerId) {
      const health = this.getHealthStatus(peerId);
      if (!health) return '';
      const parts = [];
      if (health.latency_ms !== null && health.latency_ms !== undefined) {
        parts.push(`${health.latency_ms}ms`);
      }
      if (health.last_handshake) {
        parts.push(`Last: ${this.formatLastHandshake(health.last_handshake)}`);
      }
      return parts.join(' • ');
    },
    getHealthStatusBadge(peerId) {
      const health = this.getHealthStatus(peerId);
      if (!health) return '';
      if (health.is_online) {
        if (health.latency_ms !== null && health.latency_ms !== undefined) {
          return `(${health.latency_ms}ms)`;
        }
        return '(online)';
      }
      return '(offline)';
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
    formatUpSince(timestamp) {
      if (!timestamp) return 'Unknown';
      const date = new Date(timestamp * 1000);
      return date.toLocaleString();
    },
    async handleExitNodeApply() {
      if (!this.selectedExitNode || this.selectedExitNode === this.exitNode) {
        return;
      }
      
      // Store the original selection in case we need to revert
      const originalExitNode = this.exitNode;
      const targetExitNode = this.selectedExitNode;
      
      this.loading = true;
      
      // Create a timeout promise (30 seconds)
      const timeoutPromise = new Promise((_, reject) => {
        setTimeout(() => reject(new Error('Gateway switch timed out after 30 seconds')), 30000);
      });
      
      try {
        // Race between the API call and timeout
        await Promise.race([
          this.api.update_peer_route_status('0.0.0.0/0', targetExitNode, []),
          timeoutPromise
        ]);
        
        this.exitNode = targetExitNode;
        // Reload to get updated state (with timeout)
        try {
          await Promise.race([
            this.loadExitNodeInfo(),
            // Also update health status immediately after gateway change
            this.updateHealthStatus(),
            new Promise((_, reject) => setTimeout(() => reject(new Error('Info reload timed out')), 10000))
          ]);
        } catch (reloadError) {
          console.warn('Failed to reload exit node info:', reloadError);
          // Don't fail the whole operation if reload fails
        }
      } catch (error) {
        console.error('Failed to update exit node:', error);
        // Revert selection on error
        this.selectedExitNode = originalExitNode;
        const errorMessage = error.message || error.toString();
        alert('Failed to update exit node: ' + errorMessage + '\n\nPlease try again or check the server logs.');
      } finally {
        this.loading = false;
      }
    },
    getPeerName(peerId) {
      if (!this.network || !this.network.peers || !this.network.peers[peerId]) {
        return peerId.substring(0, 8) + '...';
      }
      return this.network.peers[peerId].name || peerId.substring(0, 8) + '...';
    },
  }
}
</script>

<style scoped>
</style>

