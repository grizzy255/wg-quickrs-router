<template>
  <div class="fixed inset-0 bg-backdrop z-50 flex items-center justify-center p-4" @click.self="$emit('close')">
    <div class="bg-card rounded-lg shadow-xl w-full max-w-5xl max-h-[90vh] flex flex-col border border-divider">
      <!-- Header -->
      <div class="flex items-center justify-between px-6 py-4 border-b border-divider">
        <div class="flex items-center gap-3">
          <Route :size="24" class="text-icon" />
          <h2 class="text-xl font-semibold text-primary">Routing Tables</h2>
        </div>
        <div class="flex items-center gap-3">
          <!-- Refresh button -->
          <button 
            @click="fetchRoutingInfo" 
            :disabled="loading"
            class="px-3 py-1.5 text-sm bg-button hover:bg-button-hover text-button rounded-md flex items-center gap-2 transition-colors">
            <RefreshCw :size="16" :class="{ 'animate-spin': loading }" />
            Refresh
          </button>
          <!-- Close button -->
          <button @click="$emit('close')" class="text-secondary hover:text-primary transition-colors">
            <X :size="24" />
          </button>
        </div>
      </div>

      <!-- Content -->
      <div class="flex-1 overflow-y-auto p-6 space-y-6">
        <!-- Loading state -->
        <div v-if="loading && !routingInfo" class="flex items-center justify-center h-32">
          <div class="text-secondary">Loading routing information...</div>
        </div>

        <!-- Error state -->
        <div v-else-if="error" class="bg-red-500/10 border border-red-500/30 rounded-lg p-4">
          <p class="text-red-400">{{ error }}</p>
        </div>

        <!-- Routing data -->
        <template v-else-if="routingInfo">
          <!-- IP Rules (Policy Routing) -->
          <div class="bg-page rounded-lg border border-divider overflow-hidden">
            <div class="bg-header px-4 py-3 border-b border-divider flex items-center justify-between">
              <h3 class="font-semibold text-primary flex items-center gap-2">
                <Scale :size="18" class="text-icon" />
                IP Rules (Policy Routing)
              </h3>
              <span class="text-xs text-secondary">ip rule show</span>
            </div>
            <div class="p-4 font-mono text-xs overflow-x-auto">
              <div v-for="(rule, index) in routingInfo.ip_rules" :key="'rule-' + index" 
                   :class="getRuleClass(rule)"
                   class="py-1 px-2 rounded hover:bg-button/50 whitespace-nowrap">
                {{ rule }}
              </div>
              <div v-if="!routingInfo.ip_rules?.length" class="text-secondary">No IP rules found</div>
            </div>
          </div>

          <!-- Main Routing Table -->
          <div class="bg-page rounded-lg border border-divider overflow-hidden">
            <div class="bg-header px-4 py-3 border-b border-divider flex items-center justify-between">
              <h3 class="font-semibold text-primary flex items-center gap-2">
                <TableProperties :size="18" class="text-icon" />
                Main Routing Table
              </h3>
              <span class="text-xs text-secondary">ip route show table main</span>
            </div>
            <div class="p-4 font-mono text-xs overflow-x-auto">
              <div v-for="(route, index) in routingInfo.routes_main" :key="'main-' + index"
                   :class="getRouteClass(route)"
                   class="py-1 px-2 rounded hover:bg-button/50 whitespace-nowrap">
                {{ route }}
              </div>
              <div v-if="!routingInfo.routes_main?.length" class="text-secondary">No routes in main table</div>
            </div>
          </div>

          <!-- WireGuard Routing Tables (if any) -->
          <div v-if="routingInfo.routes_wg && Object.keys(routingInfo.routes_wg).length > 0"
               v-for="(routes, tableName) in routingInfo.routes_wg" :key="tableName"
               class="bg-page rounded-lg border border-divider overflow-hidden">
            <div class="bg-header px-4 py-3 border-b border-divider flex items-center justify-between">
              <h3 class="font-semibold text-primary flex items-center gap-2">
                <Network :size="18" class="text-blue-400" />
                Table: {{ tableName }}
              </h3>
              <span class="text-xs text-secondary">ip route show table {{ tableName }}</span>
            </div>
            <div class="p-4 font-mono text-xs overflow-x-auto">
              <div v-for="(route, index) in routes" :key="tableName + '-' + index"
                   :class="getRouteClass(route)"
                   class="py-1 px-2 rounded hover:bg-button/50 whitespace-nowrap">
                {{ route }}
              </div>
              <div v-if="!routes?.length" class="text-secondary">No routes in this table</div>
            </div>
          </div>

          <!-- WireGuard Peer AllowedIPs -->
          <div v-if="routingInfo.peer_allowed_ips && Object.keys(routingInfo.peer_allowed_ips).length > 0"
               class="bg-page rounded-lg border border-divider overflow-hidden">
            <div class="bg-header px-4 py-3 border-b border-divider flex items-center justify-between">
              <h3 class="font-semibold text-primary flex items-center gap-2">
                <Users :size="18" class="text-green-400" />
                WireGuard Peer AllowedIPs
              </h3>
              <span class="text-xs text-secondary">wg show {{ routingInfo.interface || 'wg0' }} allowed-ips</span>
            </div>
            <div class="p-4 space-y-3">
              <div v-for="(allowedIps, publicKey) in routingInfo.peer_allowed_ips" :key="publicKey"
                   class="bg-card rounded-lg p-3 border border-divider">
                <div class="flex items-start gap-3">
                  <div class="flex-1 min-w-0">
                    <div class="flex items-center gap-2 mb-2">
                      <span class="text-xs font-medium text-secondary">Public Key:</span>
                      <code class="text-xs text-primary font-mono break-all">{{ publicKey }}</code>
                    </div>
                    <div class="flex items-center gap-2">
                      <span class="text-xs font-medium text-secondary">AllowedIPs:</span>
                      <div class="flex flex-wrap gap-1">
                        <span v-for="ip in allowedIps.split(',')" :key="ip" 
                              class="px-2 py-0.5 text-xs rounded bg-blue-500/20 text-blue-400 font-mono">
                          {{ ip.trim() }}
                        </span>
                        <span v-if="!allowedIps || allowedIps === '(none)'" class="text-xs text-secondary">
                          (none)
                        </span>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <!-- Default Gateway Info -->
          <div v-if="routingInfo.default_gateway" class="bg-blue-500/10 border border-blue-500/30 rounded-lg p-4">
            <div class="flex items-start gap-3">
              <Info :size="20" class="text-blue-400 shrink-0 mt-0.5" />
              <div>
                <p class="text-sm font-medium text-primary">Default Gateway</p>
                <code class="text-sm text-blue-400">{{ routingInfo.default_gateway }}</code>
              </div>
            </div>
          </div>
        </template>
      </div>

      <!-- Footer -->
      <div class="px-6 py-3 border-t border-divider flex items-center justify-between text-sm text-secondary">
        <div v-if="lastUpdate">Last updated: {{ lastUpdate }}</div>
        <button 
          @click="$emit('close')"
          class="px-4 py-2 bg-button text-button rounded-lg hover:bg-button-hover transition-colors">
          Close
        </button>
      </div>
    </div>
  </div>
</template>

<script>
import { Route, X, RefreshCw, Scale, TableProperties, Network, Users, Info } from 'lucide-vue-next';

export default {
  name: 'RoutingTableDialog',
  components: {
    Route,
    X,
    RefreshCw,
    Scale,
    TableProperties,
    Network,
    Users,
    Info
  },
  props: {
    api: {
      type: Object,
      required: true
    }
  },
  emits: ['close'],
  data() {
    return {
      routingInfo: null,
      loading: false,
      error: null,
      lastUpdate: null
    };
  },
  methods: {
    async fetchRoutingInfo() {
      this.loading = true;
      this.error = null;
      
      try {
        const result = await this.api.get_routing_info();
        this.routingInfo = result;
        this.lastUpdate = new Date().toLocaleTimeString();
      } catch (err) {
        this.error = err.message || 'Failed to fetch routing information';
      } finally {
        this.loading = false;
      }
    },
    getRuleClass(rule) {
      if (rule.includes('lookup main')) return 'text-primary';
      if (rule.includes('lookup local')) return 'text-secondary';
      if (rule.includes('blackhole')) return 'text-red-400 bg-red-500/10';
      if (rule.includes('wg_') || rule.includes('wireguard')) return 'text-blue-400 bg-blue-500/10';
      if (/lookup \d+/.test(rule)) return 'text-green-400 bg-green-500/10';
      return 'text-primary';
    },
    getRouteClass(route) {
      if (route.includes('default')) return 'text-yellow-400 bg-yellow-500/10';
      if (route.includes('blackhole')) return 'text-red-400 bg-red-500/10';
      if (route.includes('WireStream') || route.includes('wg')) return 'text-blue-400 bg-blue-500/10';
      if (route.includes('unreachable')) return 'text-red-400';
      return 'text-primary';
    },
    handleKeydown(e) {
      if (e.key === 'Escape') {
        this.$emit('close');
      }
    }
  },
  mounted() {
    this.fetchRoutingInfo();
    document.addEventListener('keydown', this.handleKeydown);
  },
  beforeUnmount() {
    document.removeEventListener('keydown', this.handleKeydown);
  }
};
</script>
