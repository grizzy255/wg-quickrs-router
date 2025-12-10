<template>
  <div class="bg-card rounded-lg shadow-sm border border-divider p-5 hover:shadow-md transition-shadow duration-200">
    <h2 class="text-lg font-semibold text-primary mb-4 flex items-center gap-2">
      <Settings :size="20" class="text-icon" />
      Control Center
    </h2>
    <div class="space-y-2.5">
      <!-- WireGuard Toggle -->
      <div class="flex items-center justify-between">
        <span class="text-sm text-secondary flex items-center gap-2">
          <Power :size="16" class="text-icon" />
          WireGuard 
        </span>
        <div v-if="wireguardStatus === 'unknown'"
             class="inline-block align-middle shadow-md rounded-full transition-all w-5.5 h-3 bg-yellow-500 cursor-pointer hover:bg-yellow-400 focus:outline-none focus:ring-2 focus:ring-yellow-500 focus:ring-offset-2"
             role="button"
             tabindex="0"
             :aria-label="'WireGuard Networking Status Unknown'"
             :aria-pressed="false"
             title="WireGuard Networking Status Unknown">
          <div class="shadow-md rounded-full w-[8px] h-[8px] mx-[7px] my-[2px] bg-white"></div>
        </div>
        <div v-else-if="wireguardStatus === 'down'"
             class="inline-block align-middle shadow-md rounded-full transition-all w-5.5 h-3 bg-red-500 cursor-pointer hover:bg-red-400 focus:outline-none focus:ring-2 focus:ring-red-500 focus:ring-offset-2"
             role="button"
             tabindex="0"
             :aria-label="'Enable WireGuard Networking'"
             :aria-pressed="false"
             title="Enable WireGuard Networking"
             @click="$emit('toggle-wireguard')"
             @keydown.enter="$emit('toggle-wireguard')"
             @keydown.space.prevent="$emit('toggle-wireguard')">
          <div class="shadow-md rounded-full w-[8px] h-[8px] mx-[2px] my-[2px] bg-white"></div>
        </div>
        <div v-else-if="wireguardStatus === 'up'"
             class="inline-block align-middle shadow-md rounded-full transition-all w-5.5 h-3 bg-green-500 cursor-pointer hover:bg-green-400 focus:outline-none focus:ring-2 focus:ring-green-500 focus:ring-offset-2"
             role="button"
             tabindex="0"
             :aria-label="'Disable WireGuard Networking'"
             :aria-pressed="true"
             title="Disable WireGuard Networking"
             @click="$emit('toggle-wireguard')"
             @keydown.enter="$emit('toggle-wireguard')"
             @keydown.space.prevent="$emit('toggle-wireguard')">
          <div class="shadow-md rounded-full w-[8px] h-[8px] mx-[12px] my-[2px] bg-white"></div>
        </div>
      </div>
      
      <!-- Router Mode Toggle -->
      <div class="flex items-center justify-between">
        <span class="text-sm text-secondary flex items-center gap-2">
          <Router :size="16" class="text-icon" />
          Router Mode 
        </span>
        <div v-if="routerMode === 'unknown'"
             class="inline-block align-middle shadow-md rounded-full transition-all w-5.5 h-3 bg-yellow-500 cursor-pointer hover:bg-yellow-400 focus:outline-none focus:ring-2 focus:ring-yellow-500 focus:ring-offset-2"
             role="button"
             tabindex="0"
             :aria-label="'Router Mode Status Unknown'"
             :aria-pressed="false"
             title="Router Mode Status Unknown">
          <div class="shadow-md rounded-full w-[8px] h-[8px] mx-[7px] my-[2px] bg-white"></div>
        </div>
        <div v-else-if="routerMode === 'host'"
             class="inline-block align-middle shadow-md rounded-full transition-all w-5.5 h-3 bg-gray-400 cursor-pointer hover:bg-gray-500 focus:outline-none focus:ring-2 focus:ring-gray-500 focus:ring-offset-2"
             role="button"
             tabindex="0"
             :aria-label="'Switch to Router Mode'"
             :aria-pressed="false"
             title="Switch to Router Mode"
             @click="$emit('toggle-router-mode')"
             @keydown.enter="$emit('toggle-router-mode')"
             @keydown.space.prevent="$emit('toggle-router-mode')">
          <div class="shadow-md rounded-full w-[8px] h-[8px] mx-[2px] my-[2px] bg-white"></div>
        </div>
        <div v-else-if="routerMode === 'router'"
             class="inline-block align-middle shadow-md rounded-full transition-all w-5.5 h-3 bg-green-500 cursor-pointer hover:bg-green-600 focus:outline-none focus:ring-2 focus:ring-green-500 focus:ring-offset-2"
             role="button"
             tabindex="0"
             :aria-label="'Switch to Host Mode'"
             :aria-pressed="true"
             title="Switch to Host Mode"
             @click="$emit('toggle-router-mode')"
             @keydown.enter="$emit('toggle-router-mode')"
             @keydown.space.prevent="$emit('toggle-router-mode')">
          <div class="shadow-md rounded-full w-[8px] h-[8px] mx-[12px] my-[2px] bg-white"></div>
        </div>
      </div>
      
      <!-- Smart Gateway Toggle (only visible in Router Mode) -->
      <div v-if="routerMode === 'router'" class="flex items-center justify-between">
        <span class="text-sm text-secondary flex items-center gap-2">
          <Zap :size="16" class="text-icon" />
          Smart Gateway
        </span>
        <div v-if="autoFailoverLoading"
             class="inline-block align-middle shadow-md rounded-full transition-all w-5.5 h-3 bg-yellow-500 cursor-wait"
             role="button"
             tabindex="0"
             :aria-label="'Loading...'"
             title="Loading...">
          <div class="shadow-md rounded-full w-[8px] h-[8px] mx-[7px] my-[2px] bg-white animate-pulse"></div>
        </div>
        <div v-else-if="!autoFailover"
             class="inline-block align-middle shadow-md rounded-full transition-all w-5.5 h-3 bg-gray-400 cursor-pointer hover:bg-gray-500 focus:outline-none focus:ring-2 focus:ring-gray-500 focus:ring-offset-2"
             role="button"
             tabindex="0"
             :aria-label="'Enable Smart Gateway (Auto Failover)'"
             :aria-pressed="false"
             title="Enable Smart Gateway"
             @click="$emit('toggle-auto-failover')"
             @keydown.enter="$emit('toggle-auto-failover')"
             @keydown.space.prevent="$emit('toggle-auto-failover')">
          <div class="shadow-md rounded-full w-[8px] h-[8px] mx-[2px] my-[2px] bg-white"></div>
        </div>
        <div v-else
             class="inline-block align-middle shadow-md rounded-full transition-all w-5.5 h-3 bg-green-500 cursor-pointer hover:bg-green-600 focus:outline-none focus:ring-2 focus:ring-green-500 focus:ring-offset-2"
             role="button"
             tabindex="0"
             :aria-label="'Disable Smart Gateway (Auto Failover)'"
             :aria-pressed="true"
             title="Disable Smart Gateway"
             @click="$emit('toggle-auto-failover')"
             @keydown.enter="$emit('toggle-auto-failover')"
             @keydown.space.prevent="$emit('toggle-auto-failover')">
          <div class="shadow-md rounded-full w-[8px] h-[8px] mx-[12px] my-[2px] bg-white"></div>
        </div>
      </div>
      
      <!-- Connected Peers -->
      <div v-if="wireguardStatus === 'up' && connectedPeers.length > 0">
        <div class="text-sm text-secondary flex items-center gap-2 mb-1">
          <Users :size="16" class="text-icon" />
          Connected Peers
        </div>
        <div class="text-xs space-y-0.5 pl-6">
          <div v-for="peerId in connectedPeers" :key="peerId" 
               class="flex items-center justify-between">
            <span class="text-secondary font-medium">{{ getPeerName(peerId) }}</span>
            <div class="flex items-center gap-1.5">
              <span class="text-primary">{{ formatLastHandshake(getPeerLastHandshake(peerId)) }}</span>
              <!-- LAN Access toggle (only in router mode) -->
              <button 
                  v-if="routerMode === 'router'"
                  @click="$emit('toggle-lan-access', peerId)"
                  :disabled="lanAccessLoading[peerId]"
                  :class="[
                    'p-0.5 rounded transition focus:outline-none',
                    hasLanAccess(peerId) 
                      ? 'text-green-600 hover:text-green-700 hover:bg-green-50 dark:hover:bg-green-900/20' 
                      : 'text-gray-400 hover:text-gray-600 hover:bg-gray-100 dark:hover:bg-gray-700',
                    lanAccessLoading[peerId] ? 'opacity-50 cursor-not-allowed' : ''
                  ]"
                  :title="hasLanAccess(peerId) ? 'LAN Access: Enabled (click to disable)' : 'LAN Access: Disabled (click to enable)'">
                <Home :size="14" />
              </button>
              <!-- Reconnect button -->
              <button 
                  @click="$emit('peer-control', peerId, 'reconnect')"
                  :disabled="peerControlLoading[peerId]"
                  class="p-0.5 rounded hover:bg-button-hover text-secondary hover:text-blue-600 disabled:opacity-50 disabled:cursor-not-allowed transition focus:outline-none"
                  title="Reconnect peer">
                <RefreshCw :size="14" :class="{ 'animate-spin': peerControlLoading[peerId] === 'reconnect' }" />
              </button>
              <!-- Stop button -->
              <button 
                  @click="$emit('peer-control', peerId, 'stop')"
                  :disabled="peerControlLoading[peerId]"
                  class="p-0.5 rounded hover:bg-button-hover text-secondary hover:text-red-600 disabled:opacity-50 disabled:cursor-not-allowed transition focus:outline-none"
                  title="Stop peer">
                <Square :size="14" />
              </button>
            </div>
          </div>
        </div>
      </div>
      <div v-else-if="wireguardStatus === 'up'">
        <div class="text-sm text-secondary mb-1 flex items-center gap-2">
          <Users :size="16" class="text-icon" />
          Connected Peers:
        </div>
        <div class="text-sm text-muted pl-6">No active connections</div>
      </div>
    </div>
  </div>
</template>

<script>
import { Settings, Power, Users, Router, Home, RefreshCw, Square, Zap } from 'lucide-vue-next';

export default {
  name: 'ControlCenterCard',
  components: {
    Settings,
    Power,
    Users,
    Router,
    Home,
    RefreshCw,
    Square,
    Zap
  },
  props: {
    wireguardStatus: {
      type: String,
      default: 'unknown'
    },
    routerMode: {
      type: String,
      default: 'unknown'
    },
    connectedPeers: {
      type: Array,
      default: () => []
    },
    getPeerName: {
      type: Function,
      required: true
    },
    getPeerLastHandshake: {
      type: Function,
      required: true
    },
    formatLastHandshake: {
      type: Function,
      required: true
    },
    hasLanAccess: {
      type: Function,
      default: () => () => true
    },
    lanAccessLoading: {
      type: Object,
      default: () => ({})
    },
    peerControlLoading: {
      type: Object,
      default: () => ({})
    },
    autoFailover: {
      type: Boolean,
      default: false
    },
    autoFailoverLoading: {
      type: Boolean,
      default: false
    }
  },
  emits: ['toggle-wireguard', 'toggle-router-mode', 'toggle-lan-access', 'peer-control', 'toggle-auto-failover']
}
</script>

