<template>
  <div class="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50" @click.self="$emit('close')">
    <div class="bg-card border border-divider rounded-xl shadow-2xl w-full max-w-4xl max-h-[85vh] overflow-hidden flex flex-col">
      <!-- Header -->
      <div class="bg-header border-b border-divider px-6 py-4 flex items-center justify-between">
        <div class="flex items-center gap-3">
          <div class="p-2 bg-blue-500/20 rounded-lg">
            <Code :size="24" class="text-blue-400" />
          </div>
          <div>
            <h2 class="text-xl font-bold text-primary">API Reference</h2>
            <p class="text-sm text-secondary">REST API endpoints for wg-quickrs</p>
          </div>
        </div>
        <button @click="$emit('close')" class="p-2 hover:bg-button rounded-lg transition-colors">
          <X :size="20" class="text-secondary" />
        </button>
      </div>

      <!-- Content -->
      <div class="flex-1 overflow-y-auto p-6">
        <!-- Search -->
        <div class="mb-6">
          <div class="relative">
            <Search :size="18" class="absolute left-3 top-1/2 -translate-y-1/2 text-secondary" />
            <input 
              v-model="search" 
              type="text" 
              placeholder="Search endpoints..."
              class="w-full pl-10 pr-4 py-2.5 bg-input border border-divider rounded-lg text-primary placeholder-secondary focus:outline-none focus:ring-2 focus:ring-blue-500/50"
            />
          </div>
        </div>

        <!-- Endpoint Categories -->
        <div class="space-y-6">
          <div v-for="category in filteredCategories" :key="category.name" class="bg-page rounded-lg border border-divider overflow-hidden">
            <div class="bg-header px-4 py-3 border-b border-divider">
              <h3 class="font-semibold text-primary flex items-center gap-2">
                <component :is="category.icon" :size="18" class="text-icon" />
                {{ category.name }}
              </h3>
            </div>
            <div class="divide-y divide-divider">
              <div v-for="endpoint in category.endpoints" :key="endpoint.path" class="px-4 py-3 hover:bg-button/30 transition-colors">
                <div class="flex items-start gap-3">
                  <span :class="methodClass(endpoint.method)" class="px-2 py-0.5 text-xs font-bold rounded uppercase shrink-0 mt-0.5">
                    {{ endpoint.method }}
                  </span>
                  <div class="flex-1 min-w-0">
                    <code class="text-sm text-primary font-mono break-all">{{ endpoint.path }}</code>
                    <p class="text-sm text-secondary mt-1">{{ endpoint.description }}</p>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- Base URL Info -->
        <div class="mt-6 p-4 bg-blue-500/10 border border-blue-500/30 rounded-lg">
          <div class="flex items-start gap-3">
            <Info :size="20" class="text-blue-400 shrink-0 mt-0.5" />
            <div>
              <p class="text-sm text-primary font-medium">Base URL</p>
              <code class="text-sm text-blue-400">http://{{ host }}</code>
              <p class="text-xs text-secondary mt-2">All endpoints require authentication via Bearer token (except /api/init/* and /api/token).</p>
            </div>
          </div>
        </div>
      </div>

      <!-- Footer -->
      <div class="bg-header border-t border-divider px-6 py-3 flex justify-end">
        <button 
          @click="$emit('close')"
          class="px-4 py-2 bg-button text-text-button rounded-lg hover:bg-button-hover transition-colors">
          Close
        </button>
      </div>
    </div>
  </div>
</template>

<script>
import { Code, X, Search, Info, Server, Network, Router, Shield, Users, Zap, Key } from 'lucide-vue-next';

export default {
  name: 'ApiReferenceDialog',
  components: { Code, X, Search, Info, Server, Network, Router, Shield, Users, Zap, Key },
  emits: ['close'],
  data() {
    return {
      search: '',
      categories: [
        {
          name: 'System',
          icon: 'Server',
          endpoints: [
            { method: 'GET', path: '/api/version', description: 'Get application version and build info' }
          ]
        },
        {
          name: 'Network Configuration',
          icon: 'Network',
          endpoints: [
            { method: 'GET', path: '/api/network/summary', description: 'Get network summary, peers, and telemetry data. Optional: ?only_digest=true' },
            { method: 'PATCH', path: '/api/network/config', description: 'Update network configuration' },
            { method: 'POST', path: '/api/network/reserve/address', description: 'Reserve an IP address in the network' }
          ]
        },
        {
          name: 'WireGuard',
          icon: 'Shield',
          endpoints: [
            { method: 'POST', path: '/api/wireguard/status', description: 'Start or stop the WireGuard tunnel' }
          ]
        },
        {
          name: 'Mode (Host/Router)',
          icon: 'Router',
          endpoints: [
            { method: 'GET', path: '/api/mode', description: 'Get current operating mode (host or router)' },
            { method: 'PATCH', path: '/api/mode/toggle', description: 'Switch between Host and Router mode' },
            { method: 'GET', path: '/api/mode/can-switch', description: 'Check if mode can be switched (requires no peers)' },
            { method: 'PATCH', path: '/api/mode/peer-route-status', description: 'Set exit node for default route' },
            { method: 'GET', path: '/api/mode/exit-node', description: 'Get current exit node, health status, and peers with default route' }
          ]
        },
        {
          name: 'Peer Management',
          icon: 'Users',
          endpoints: [
            { method: 'POST', path: '/api/peer/control', description: 'Control peer connection (reconnect, stop, start)' },
            { method: 'GET', path: '/api/peer/lan-access', description: 'Get LAN access status for all peers' },
            { method: 'PATCH', path: '/api/peer/lan-access', description: 'Enable or disable LAN access for a peer' }
          ]
        },
        {
          name: 'Smart Gateway',
          icon: 'Zap',
          endpoints: [
            { method: 'GET', path: '/api/router-mode/auto-failover', description: 'Get Smart Gateway (auto-failover) status' },
            { method: 'POST', path: '/api/router-mode/auto-failover', description: 'Enable or disable automatic gateway failover' }
          ]
        },
        {
          name: 'Authentication',
          icon: 'Key',
          endpoints: [
            { method: 'POST', path: '/api/token', description: 'Authenticate and get access token' }
          ]
        },
        {
          name: 'Initialization',
          icon: 'Server',
          endpoints: [
            { method: 'GET', path: '/api/init/status', description: 'Check if application is initialized (no auth required)' },
            { method: 'GET', path: '/api/init/info', description: 'Get server info for initialization (no auth required)' },
            { method: 'POST', path: '/api/init', description: 'Initialize the application with config (no auth required)' }
          ]
        }
      ]
    };
  },
  computed: {
    host() {
      return window.location.host;
    },
    filteredCategories() {
      if (!this.search.trim()) {
        return this.categories;
      }
      const searchLower = this.search.toLowerCase();
      return this.categories
        .map(cat => ({
          ...cat,
          endpoints: cat.endpoints.filter(ep => 
            ep.path.toLowerCase().includes(searchLower) ||
            ep.description.toLowerCase().includes(searchLower) ||
            ep.method.toLowerCase().includes(searchLower)
          )
        }))
        .filter(cat => cat.endpoints.length > 0);
    }
  },
  methods: {
    methodClass(method) {
      const classes = {
        'GET': 'bg-green-500/20 text-green-400',
        'POST': 'bg-blue-500/20 text-blue-400',
        'PATCH': 'bg-amber-500/20 text-amber-400',
        'PUT': 'bg-purple-500/20 text-purple-400',
        'DELETE': 'bg-red-500/20 text-red-400'
      };
      return classes[method] || 'bg-gray-500/20 text-gray-400';
    }
  }
};
</script>

