<template>
  <div class="bg-card rounded-lg shadow-sm border border-divider p-5 hover:shadow-md transition-shadow duration-200">
    <h2 class="text-lg font-semibold text-primary mb-4 flex items-center gap-2">
      <Activity :size="20" class="text-icon" />
      System Health & Info.
    </h2>
    <div class="space-y-2.5">
      <!-- Web Server Status -->
      <div class="flex items-center justify-between">
        <span class="text-sm text-secondary flex items-center gap-2">
          <Server :size="16" class="text-icon" />
          Web Server Status
        </span>
        <span :class="[
          webServerStatus === 'up' ? 'bg-badge-success-bg text-badge-success-text border-badge-success-border' : 
          webServerStatus === 'down' ? 'bg-badge-error-bg text-badge-error-text border-badge-error-border' : 
          'bg-badge-warning-bg text-badge-warning-text border-badge-warning-border'
        ]" class="px-2 py-1 text-xs font-medium rounded-full border border-opacity-30">
          {{ webServerStatus === 'up' ? 'up' : webServerStatus === 'down' ? 'down' : 'unknown' }}
        </span>
      </div>
      <!-- WireGuard Status -->
      <div class="flex items-center justify-between">
        <span class="text-sm text-secondary flex items-center gap-2">
          <Shield :size="16" class="text-icon" />
          WireGuard Status
        </span>
        <span :class="[
          wireguardStatus === 'up' ? 'bg-badge-success-bg text-badge-success-text border-badge-success-border' : 
          wireguardStatus === 'down' ? 'bg-badge-error-bg text-badge-error-text border-badge-error-border' : 
          'bg-badge-warning-bg text-badge-warning-text border-badge-warning-border'
        ]" class="px-2 py-1 text-xs font-medium rounded-full border border-opacity-30">
          {{ wireguardStatus === 'up' ? 'up' : wireguardStatus === 'down' ? 'down' : 'unknown' }}
        </span>
      </div>
      
      <!-- Network Information -->
      <div v-if="network && network.this_peer" class="space-y-2.5">
        <div class="flex items-center justify-between">
          <span class="text-sm text-secondary flex items-center gap-2">
            <Globe :size="16" class="text-icon" />
            Network Name
          </span>
          <span class="text-sm font-medium text-primary text-right">
            {{ network.name }}
          </span>
        </div>
        <div class="flex items-center justify-between">
          <span class="text-sm text-secondary flex items-center gap-2">
            <Home :size="16" class="text-icon" />
            Tunnel IP
          </span>
          <span class="text-sm font-medium text-primary text-right">
            {{ network.peers[network.this_peer].address }}
          </span>
        </div>
        <div class="flex items-center justify-between">
          <span class="text-sm text-secondary flex items-center gap-2">
            <Link :size="16" class="text-icon" />
            Endpoint
          </span>
          <span class="text-sm font-medium text-primary text-right">
            {{ stringifyEndpoint(network.peers[network.this_peer].endpoint) || 'N/A' }}
          </span>
        </div>
        <!-- LAN Subnet (only in router mode) -->
        <div v-if="routerMode === 'router'" class="flex items-center justify-between">
          <span class="text-sm text-secondary flex items-center gap-2">
            <Network :size="16" class="text-icon" />
            LAN Subnet
          </span>
          <div class="flex items-center gap-1.5">
            <span v-if="!isEditingLan" class="text-sm font-medium text-primary text-right">
              {{ lanCidr || 'Not configured' }}
            </span>
            <input 
                v-else
                ref="lanInput"
                v-model="editLanValue"
                @keydown.enter="saveLanCidr"
                @keydown.escape="cancelEditLan"
                @blur="saveLanCidr"
                class="text-sm font-medium text-primary text-right bg-transparent border-b border-blue-500 focus:outline-none w-48"
                placeholder="192.168.1.0/24,10.0.0.0/8"
            />
            <button 
                @click="isEditingLan ? saveLanCidr() : startEditLan()"
                class="p-0.5 rounded hover:bg-button-hover text-secondary hover:text-blue-600 transition focus:outline-none"
                :title="isEditingLan ? 'Save' : 'Edit LAN subnet'">
              <Check v-if="isEditingLan" :size="14" />
              <Pencil v-else :size="14" />
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script>
import { Activity, Server, Shield, Globe, Home, Link, Network, Pencil, Check } from 'lucide-vue-next';

export default {
  name: 'SystemHealthCard',
  components: {
    Activity,
    Server,
    Shield,
    Globe,
    Home,
    Link,
    Network,
    Pencil,
    Check
  },
  props: {
    webServerStatus: {
      type: String,
      default: 'unknown'
    },
    wireguardStatus: {
      type: String,
      default: 'unknown'
    },
    network: {
      type: Object,
      default: () => ({})
    },
    stringifyEndpoint: {
      type: Function,
      required: true
    },
    routerMode: {
      type: String,
      default: 'unknown'
    },
    lanCidr: {
      type: String,
      default: null
    }
  },
  emits: ['update-lan-cidr'],
  data() {
    return {
      isEditingLan: false,
      editLanValue: ''
    }
  },
  methods: {
    startEditLan() {
      this.editLanValue = this.lanCidr || '';
      this.isEditingLan = true;
      this.$nextTick(() => {
        this.$refs.lanInput?.focus();
        this.$refs.lanInput?.select();
      });
    },
    saveLanCidr() {
      if (this.editLanValue && this.editLanValue !== this.lanCidr) {
        // Validate CIDR format (supports comma-separated multiple CIDRs)
        const cidrRegex = /^(\d{1,3}\.){3}\d{1,3}\/\d{1,2}$/;
        const cidrs = this.editLanValue.split(',').map(c => c.trim()).filter(c => c);
        
        if (cidrs.length === 0) {
          alert('Please enter at least one CIDR');
          return;
        }
        
        for (const cidr of cidrs) {
          if (!cidrRegex.test(cidr)) {
            alert(`Invalid CIDR format: "${cidr}". Please use format like 192.168.1.0/24`);
            return;
          }
        }
        
        // Normalize: join with comma and space
        const normalizedValue = cidrs.join(', ');
        this.$emit('update-lan-cidr', normalizedValue);
      }
      this.isEditingLan = false;
    },
    cancelEditLan() {
      this.isEditingLan = false;
      this.editLanValue = '';
    }
  }
}
</script>
