<template>
  <div class="fixed inset-0 flex items-center justify-center bg-backdrop z-50 font-mono">
    <div class="bg-card rounded-lg shadow-sm border border-divider max-w-4xl w-full mx-4 max-h-[90vh] overflow-y-auto flex flex-col">
      <div class="p-6 flex-1">
        <h2 class="text-2xl font-semibold mb-4 text-primary">Initialize wg-quickrs</h2>
        <p class="text-sm text-secondary mb-6">Please provide the following information to set up your WireGuard network.</p>

        <!-- Progress indicator -->
        <div class="mb-6">
          <div class="flex items-center justify-between mb-2">
            <span class="text-sm font-medium text-primary">Step {{ currentStep }} of {{ totalSteps }}</span>
            <span class="text-sm text-secondary">{{ Math.round((currentStep / totalSteps) * 100) }}%</span>
          </div>
          <div class="w-full bg-button rounded-full h-2">
            <div class="bg-blue-600 h-2 rounded-full transition-all duration-300" 
                 :style="{ width: (currentStep / totalSteps) * 100 + '%' }"></div>
          </div>
        </div>

        <!-- Step 1: Network Settings -->
        <div v-if="currentStep === 1" class="space-y-6">
          <div class="bg-card rounded-lg shadow-sm border border-divider p-5">
            <h3 class="text-lg font-semibold mb-4 text-primary">Network Settings</h3>
            
            <input-field v-model="formData.network_name" 
                         :value-prev="formData.network_name"
                         label="Network Name"
                         placeholder="wg-quickrs-home"
                         class="w-full"/>

            <input-field v-model="formData.network_subnet" 
                         :value-prev="formData.network_subnet"
                         label="Network Subnet (CIDR)"
                         placeholder="10.0.34.0/24"
                         :input-color="networkSubnetValid ? null : 'enabled:bg-red-200'"
                         class="w-full"
                         @input="validateNetworkSubnet"/>
            <p v-if="!networkSubnetValid && formData.network_subnet" class="text-xs text-error mt-1 ml-2">
              Invalid CIDR format. Please use format like 10.0.34.0/24
            </p>
          </div>
        </div>

        <!-- Step 2: Web Server Settings -->
        <div v-if="currentStep === 2" class="space-y-6">
          <div class="bg-card rounded-lg shadow-sm border border-divider p-5">
            <h3 class="text-lg font-semibold mb-4 text-primary">Web Server Settings</h3>
          
          <div class="my-0.5 truncate flex items-center relative ml-2">
            <field field="Web Server Address:" class="mr-1"></field>
            <select v-model="formData.agent_web_address" 
                    class="rounded pl-1.5 pt-[2px] pb-[2px] my-0.5 focus:outline-none focus:ring-0 border-1 border-input focus:border-input-focus outline-none w-full text-lg text-secondary grow bg-input">
              <option value="">Select interface...</option>
              <option v-for="iface in initInfo.interfaces" :key="iface.name" :value="iface.ip">
                {{ iface.name }} ({{ iface.ip }})<span v-if="iface.recommended"> - Recommended</span>
              </option>
            </select>
          </div>

          <div class="my-0.5 truncate flex items-center relative ml-2">
            <checkbox :checked="formData.agent_web_http_enabled" 
                      label="Enable HTTP"
                      @click="formData.agent_web_http_enabled = !formData.agent_web_http_enabled"/>
          </div>

          <div v-if="formData.agent_web_http_enabled">
            <input-field v-model.number="formData.agent_web_http_port" 
                         :value-prev="formData.agent_web_http_port"
                         label="HTTP Port"
                         placeholder="80"
                         class="w-full"/>
          </div>

          <div class="my-0.5 truncate flex items-center relative ml-2">
            <checkbox :checked="formData.agent_web_https_enabled" 
                      label="Enable HTTPS"
                      @click="formData.agent_web_https_enabled = !formData.agent_web_https_enabled"/>
          </div>

          <div v-if="formData.agent_web_https_enabled" class="space-y-3">
            <input-field v-model.number="formData.agent_web_https_port" 
                         :value-prev="formData.agent_web_https_port"
                         label="HTTPS Port"
                         placeholder="443"
                         class="w-full"/>

            <input-field v-model="formData.agent_web_https_tls_cert" 
                         :value-prev="formData.agent_web_https_tls_cert"
                         label="HTTPS TLS Certificate"
                         placeholder="certs/servers/0.0.0.0/cert.pem"
                         class="w-full"/>

            <input-field v-model="formData.agent_web_https_tls_key" 
                         :value-prev="formData.agent_web_https_tls_key"
                         label="HTTPS TLS Key"
                         placeholder="certs/servers/0.0.0.0/key.pem"
                         class="w-full"/>
          </div>

          <div class="my-0.5 truncate flex items-center relative ml-2">
            <checkbox :checked="formData.agent_web_password_enabled" 
                      label="Enable Password Authentication"
                      @click="formData.agent_web_password_enabled = !formData.agent_web_password_enabled"/>
          </div>

          <div v-if="formData.agent_web_password_enabled" class="my-0.5 truncate flex items-center relative ml-2">
            <field field="Password:" class="mr-1"></field>
            <input v-model="formData.agent_web_password_value" 
                   type="password"
                   placeholder="Enter password"
                   class="rounded pl-1.5 pt-[2px] pb-[2px] my-0.5 focus:outline-none focus:ring-0 border-1 border-input focus:border-input-focus outline-none w-full text-lg text-secondary grow bg-input"/>
          </div>
          </div>
        </div>

        <!-- Step 3: VPN & Firewall Settings -->
        <div v-if="currentStep === 3" class="space-y-6">
          <div class="bg-card rounded-lg shadow-sm border border-divider p-5">
            <h3 class="text-lg font-semibold mb-4 text-primary">VPN Settings</h3>
            
            <div class="my-0.5 truncate flex items-center relative ml-2">
              <checkbox :checked="formData.agent_vpn_enabled" 
                        label="Enable VPN Server"
                        @click="formData.agent_vpn_enabled = !formData.agent_vpn_enabled"/>
            </div>

            <div v-if="formData.agent_vpn_enabled">
              <input-field v-model.number="formData.agent_vpn_port" 
                           :value-prev="formData.agent_vpn_port"
                           label="VPN Port"
                           placeholder="51820"
                           class="w-full"/>
            </div>
          </div>
          
          <div class="bg-card rounded-lg shadow-sm border border-divider p-5">
            <h3 class="text-lg font-semibold mb-4 text-primary">Firewall Settings</h3>
            
            <div class="my-0.5 truncate flex items-center relative ml-2">
              <checkbox :checked="formData.agent_firewall_enabled" 
                        label="Enable Firewall"
                        @click="formData.agent_firewall_enabled = !formData.agent_firewall_enabled"/>
            </div>

            <div v-if="formData.agent_firewall_enabled">
              <input-field v-model="formData.agent_firewall_utility" 
                           :value-prev="formData.agent_firewall_utility"
                           label="Firewall Utility"
                           placeholder="/usr/sbin/iptables"
                           class="w-full"/>

              <input-field v-model="formData.agent_firewall_gateway" 
                           :value-prev="formData.agent_firewall_gateway"
                           label="Firewall Gateway (Interface Name)"
                           placeholder="eth0"
                           class="w-full"/>
            </div>
          </div>
        </div>

        <!-- Step 4: Agent Peer Settings -->
        <div v-if="currentStep === 4" class="space-y-6">
          <div class="bg-card rounded-lg shadow-sm border border-divider p-5">
            <h3 class="text-lg font-semibold mb-4 text-primary">Agent Peer Settings</h3>
          
          <input-field v-model="formData.agent_peer_name" 
                       :value-prev="formData.agent_peer_name"
                       label="Peer Name"
                       placeholder="wg-quickrs-host"
                       class="w-full"/>

          <input-field v-model="formData.agent_peer_vpn_internal_address" 
                       :value-prev="formData.agent_peer_vpn_internal_address"
                       label="Internal VPN Address"
                       placeholder="10.0.34.1"
                       :input-color="vpnAddressValid ? null : 'enabled:bg-red-200'"
                       class="w-full"
                       @input="validateVpnAddress"/>
            <p v-if="!vpnAddressValid && formData.agent_peer_vpn_internal_address" class="text-xs text-error mt-1 ml-2">
              Internal VPN Address must be within the Network Subnet ({{ formData.network_subnet }}) range
            </p>

          <input-field v-model="formData.agent_peer_vpn_endpoint" 
                       :value-prev="formData.agent_peer_vpn_endpoint"
                       label="VPN Endpoint (IP/FQDN:PORT)"
                       placeholder="192.168.1.198:51820"
                       class="w-full"/>

          <input-field v-model="formData.agent_peer_kind" 
                       :value-prev="formData.agent_peer_kind"
                       label="Peer Kind"
                       placeholder="server"
                       class="w-full"/>

          <div class="my-0.5 truncate flex items-center relative ml-2">
            <checkbox :checked="formData.agent_peer_icon_enabled" 
                      label="Enable Peer Icon"
                      @click="formData.agent_peer_icon_enabled = !formData.agent_peer_icon_enabled"/>
          </div>

          <div v-if="formData.agent_peer_icon_enabled">
            <input-field v-model="formData.agent_peer_icon_src" 
                         :value-prev="formData.agent_peer_icon_src"
                         label="Peer Icon Source"
                         placeholder=""
                         class="w-full"/>
          </div>

          <div class="my-0.5 truncate flex items-center relative ml-2">
            <checkbox :checked="formData.agent_peer_dns_enabled" 
                      label="Enable DNS"
                      @click="formData.agent_peer_dns_enabled = !formData.agent_peer_dns_enabled"/>
          </div>

          <div v-if="formData.agent_peer_dns_enabled" class="space-y-2">
            <div v-for="(dns, index) in formData.agent_peer_dns_addresses" :key="index" class="flex items-center gap-2">
              <input-field v-model="formData.agent_peer_dns_addresses[index]" 
                           :value-prev="formData.agent_peer_dns_addresses[index]"
                           :label="index === 0 ? 'DNS Address' : ''"
                           placeholder="1.1.1.1"
                           class="flex-1"/>
              <button v-if="formData.agent_peer_dns_addresses.length > 1" 
                      @click="formData.agent_peer_dns_addresses.splice(index, 1)"
                      class="px-2 py-1 text-sm bg-badge-error-bg text-badge-error-text rounded hover:bg-badge-error-bg">
                Remove
              </button>
            </div>
            <button @click="formData.agent_peer_dns_addresses.push('')"
                    class="px-3 py-1 text-sm bg-badge-info-bg text-badge-info-text rounded hover:bg-badge-info-bg">
              + Add DNS Address
            </button>
          </div>

          <div class="my-0.5 truncate flex items-center relative ml-2">
            <checkbox :checked="formData.agent_peer_mtu_enabled" 
                      label="Enable MTU"
                      @click="formData.agent_peer_mtu_enabled = !formData.agent_peer_mtu_enabled"/>
          </div>

          <div v-if="formData.agent_peer_mtu_enabled">
            <input-field v-model.number="formData.agent_peer_mtu_value" 
                         :value-prev="formData.agent_peer_mtu_value"
                         label="MTU Value"
                         placeholder="1420"
                         class="w-full"/>
          </div>
          </div>
        </div>

        <!-- Step 5: Default Peer/Connection Settings -->
        <div v-if="currentStep === 5" class="space-y-6">
          <div class="bg-card rounded-lg shadow-sm border border-divider p-5">
            <h3 class="text-lg font-semibold mb-4 text-primary">Default Settings for New Peers</h3>
          
          <input-field v-model="formData.default_peer_kind" 
                       :value-prev="formData.default_peer_kind"
                       label="Default Peer Kind"
                       placeholder="laptop"
                       class="w-full"/>

          <div class="my-0.5 truncate flex items-center relative ml-2">
            <checkbox :checked="formData.default_peer_icon_enabled" 
                      label="Enable Default Peer Icon"
                      @click="formData.default_peer_icon_enabled = !formData.default_peer_icon_enabled"/>
          </div>

          <div v-if="formData.default_peer_icon_enabled">
            <input-field v-model="formData.default_peer_icon_src" 
                         :value-prev="formData.default_peer_icon_src"
                         label="Default Peer Icon Source"
                         placeholder=""
                         class="w-full"/>
          </div>

          <div class="my-0.5 truncate flex items-center relative ml-2">
            <checkbox :checked="formData.default_peer_dns_enabled" 
                      label="Enable Default DNS"
                      @click="formData.default_peer_dns_enabled = !formData.default_peer_dns_enabled"/>
          </div>

          <div v-if="formData.default_peer_dns_enabled" class="space-y-2">
            <div v-for="(dns, index) in formData.default_peer_dns_addresses" :key="index" class="flex items-center gap-2">
              <input-field v-model="formData.default_peer_dns_addresses[index]" 
                           :value-prev="formData.default_peer_dns_addresses[index]"
                           :label="index === 0 ? 'Default DNS Address' : ''"
                           placeholder="1.1.1.1"
                           class="flex-1"/>
              <button v-if="formData.default_peer_dns_addresses.length > 1" 
                      @click="formData.default_peer_dns_addresses.splice(index, 1)"
                      class="px-2 py-1 text-sm bg-badge-error-bg text-badge-error-text rounded hover:bg-badge-error-bg">
                Remove
              </button>
            </div>
            <button @click="formData.default_peer_dns_addresses.push('')"
                    class="px-3 py-1 text-sm bg-badge-info-bg text-badge-info-text rounded hover:bg-badge-info-bg">
              + Add DNS Address
            </button>
          </div>

          <div class="my-0.5 truncate flex items-center relative ml-2">
            <checkbox :checked="formData.default_peer_mtu_enabled" 
                      label="Enable Default MTU"
                      @click="formData.default_peer_mtu_enabled = !formData.default_peer_mtu_enabled"/>
          </div>

          <div v-if="formData.default_peer_mtu_enabled">
            <input-field v-model.number="formData.default_peer_mtu_value" 
                         :value-prev="formData.default_peer_mtu_value"
                         label="Default MTU Value"
                         placeholder="1420"
                         class="w-full"/>
          </div>

          </div>
          
          <div class="bg-card rounded-lg shadow-sm border border-divider p-5">
            <h3 class="text-lg font-semibold mb-4 text-primary">Default Connection Settings</h3>
            
            <div class="my-0.5 truncate flex items-center relative ml-2">
              <checkbox :checked="formData.default_connection_persistent_keepalive_enabled" 
                        label="Enable Default PersistentKeepalive"
                        @click="formData.default_connection_persistent_keepalive_enabled = !formData.default_connection_persistent_keepalive_enabled"/>
            </div>

            <div v-if="formData.default_connection_persistent_keepalive_enabled">
              <input-field v-model.number="formData.default_connection_persistent_keepalive_period" 
                           :value-prev="formData.default_connection_persistent_keepalive_period"
                           label="Default PersistentKeepalive Period (seconds)"
                           placeholder="25"
                           class="w-full"/>
            </div>
          </div>
        </div>

        <!-- Error message -->
        <div v-if="errorMessage" class="mt-4 bg-badge-error-bg rounded-lg shadow-sm border border-badge-error-border p-5">
          <div class="text-sm text-badge-error-text">{{ errorMessage }}</div>
        </div>

        <!-- Navigation buttons -->
        <div class="flex justify-between mt-6 pt-4 border-t border-divider">
          <button v-if="currentStep > 1"
                  @click="currentStep--"
                  class="px-4 py-2 bg-button text-primary rounded hover:bg-button-hover text-sm">
            Previous
          </button>
          <div v-else></div>
          
          <button v-if="currentStep < totalSteps"
                  @click="handleNext"
                  :disabled="(currentStep === 1 && !networkSubnetValid) || (currentStep === 4 && !vpnAddressValid)"
                  class="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 disabled:bg-gray-400 disabled:text-gray-600 disabled:cursor-not-allowed text-sm">
            Next
          </button>
          <button v-else
                  @click="submitInit"
                  :disabled="isSubmitting"
                  class="px-4 py-2 bg-green-600 text-white rounded hover:bg-green-700 disabled:bg-gray-400 disabled:text-gray-600 text-sm">
            {{ isSubmitting ? 'Initializing...' : 'Complete Setup' }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script>
import InputField from "@/src/components/ui/input-field.vue";
import Field from "@/src/components/ui/field.vue";
import Checkbox from "@/src/components/ui/checkbox.vue";

export default {
  name: "init-wizard",
  components: {
    InputField,
    Field,
    Checkbox
  },
  props: {
    api: {
      type: Object,
      required: true
    }
  },
  emits: ['complete', 'cancel'],
  data() {
    return {
      currentStep: 1,
      totalSteps: 5,
      isSubmitting: false,
      errorMessage: '',
      networkSubnetValid: true,
      vpnAddressValid: true,
      initInfo: {
        interfaces: [],
        recommended_interface: null
      },
      formData: {
        // Network settings (defaults from backend)
        network_name: 'wg-quickrs-home',
        network_subnet: '10.0.34.0/24',
        
        // Web server settings (defaults from backend)
        agent_web_address: '',
        agent_web_http_enabled: false,  // default: false - user must enable
        agent_web_http_port: 80,
        agent_web_https_enabled: false,  // default: false - user must enable
        agent_web_https_port: 443,
        agent_web_https_tls_cert: '',
        agent_web_https_tls_key: '',
        agent_web_password_enabled: false,  // default: false - user must enable
        agent_web_password_value: '',
        
        // VPN settings (defaults from backend)
        agent_vpn_enabled: true,  // default: true
        agent_vpn_port: 51820,
        
        // Firewall settings (defaults from backend)
        agent_firewall_enabled: true,  // default: true
        agent_firewall_utility: '/usr/sbin/iptables',
        agent_firewall_gateway: '',
        
        // Agent peer settings (defaults from backend)
        agent_peer_name: 'wg-quickrs-host',
        agent_peer_vpn_internal_address: '10.0.34.1',  // first host in subnet
        agent_peer_vpn_endpoint: '',  // will be set from interface IP
        agent_peer_kind: 'server',  // default: "server"
        agent_peer_icon_enabled: false,  // default: false
        agent_peer_icon_src: '',
        agent_peer_dns_enabled: true,  // default: true (if addresses provided)
        agent_peer_dns_addresses: ['1.1.1.1'],  // default: ["1.1.1.1"]
        agent_peer_mtu_enabled: false,  // default: false
        agent_peer_mtu_value: 1420,
        
        // Default peer settings (defaults from backend)
        default_peer_kind: 'laptop',  // default: "laptop"
        default_peer_icon_enabled: false,  // default: false
        default_peer_icon_src: '',
        default_peer_dns_enabled: true,  // default: true (if addresses provided)
        default_peer_dns_addresses: ['1.1.1.1'],  // default: ["1.1.1.1"]
        default_peer_mtu_enabled: false,  // default: false
        default_peer_mtu_value: 1420,
        
        // Default connection settings (defaults from backend)
        default_connection_persistent_keepalive_enabled: true,  // default: true
        default_connection_persistent_keepalive_period: 25,
      }
    }
  },
  async mounted() {
    await this.loadInitInfo();
    this.setDefaults();
    // Validate initial values
    this.validateNetworkSubnet();
    this.validateVpnAddress();
  },
  methods: {
    async loadInitInfo() {
      try {
        const info = await this.api.get_init_info();
        this.initInfo = info;
      } catch (err) {
        console.error('Failed to load init info:', err);
        this.errorMessage = 'Failed to load initialization information: ' + (err.message || err);
      }
    },
    setDefaults() {
      // Set recommended interface as default web address
      if (this.initInfo.recommended_interface) {
        this.formData.agent_web_address = this.initInfo.recommended_interface.ip;
        // Set default endpoint from interface IP
        this.formData.agent_peer_vpn_endpoint = `${this.initInfo.recommended_interface.ip}:51820`;
        // Set firewall gateway to interface name
        this.formData.agent_firewall_gateway = this.initInfo.recommended_interface.name;
      }
      
      // Calculate default internal address from subnet
      if (this.formData.network_subnet) {
        try {
          // Parse subnet and get first host
          const parts = this.formData.network_subnet.split('/');
          if (parts.length === 2) {
            const ipParts = parts[0].split('.');
            if (ipParts.length === 4) {
              ipParts[3] = '1';  // First host
              this.formData.agent_peer_vpn_internal_address = ipParts.join('.');
            }
          }
        } catch (e) {
          // Keep default
        }
      }
    },
    validateNetworkSubnet() {
      if (!this.formData.network_subnet || this.formData.network_subnet.trim() === '') {
        this.networkSubnetValid = false;
        return;
      }
      
      // CIDR validation regex: IP address followed by / and subnet mask (0-32)
      const cidrRegex = /^((25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\/([0-9]|[1-2][0-9]|3[0-2])$/;
      this.networkSubnetValid = cidrRegex.test(this.formData.network_subnet.trim());
      
      // Additional validation: check if IP and subnet are valid
      if (this.networkSubnetValid) {
        const parts = this.formData.network_subnet.trim().split('/');
        const subnet = parseInt(parts[1]);
        
        // Validate subnet mask is reasonable (not 0 or too large)
        if (subnet === 0 || subnet > 32) {
          this.networkSubnetValid = false;
        }
      }
      
      // If CIDR changes and is valid, re-validate VPN address
      if (this.networkSubnetValid) {
        this.validateVpnAddress();
      }
    },
    validateVpnAddress() {
      if (!this.formData.agent_peer_vpn_internal_address || this.formData.agent_peer_vpn_internal_address.trim() === '') {
        this.vpnAddressValid = false;
        return;
      }
      
      // First check if network subnet is valid
      if (!this.networkSubnetValid || !this.formData.network_subnet) {
        this.vpnAddressValid = false;
        return;
      }
      
      try {
        const vpnIp = this.formData.agent_peer_vpn_internal_address.trim();
        const cidr = this.formData.network_subnet.trim();
        
        // Parse CIDR
        const [networkIp, prefixLength] = cidr.split('/');
        const prefix = parseInt(prefixLength);
        
        if (isNaN(prefix) || prefix < 0 || prefix > 32) {
          this.vpnAddressValid = false;
          return;
        }
        
        // Convert IPs to numbers
        const ipToNumber = (ip) => {
          const parts = ip.split('.').map(Number);
          if (parts.length !== 4 || parts.some(p => isNaN(p) || p < 0 || p > 255)) {
            return null;
          }
          return (parts[0] << 24) + (parts[1] << 16) + (parts[2] << 8) + parts[3];
        };
        
        const networkNum = ipToNumber(networkIp);
        const vpnNum = ipToNumber(vpnIp);
        
        if (networkNum === null || vpnNum === null) {
          this.vpnAddressValid = false;
          return;
        }
        
        // Calculate network mask
        const mask = (0xFFFFFFFF << (32 - prefix)) >>> 0;
        
        // Check if VPN IP is in the network range
        const networkStart = networkNum & mask;
        const networkEnd = networkStart | (~mask >>> 0);
        
        this.vpnAddressValid = vpnNum >= networkStart && vpnNum <= networkEnd;
      } catch (e) {
        this.vpnAddressValid = false;
      }
    },
    validateStep() {
      this.errorMessage = '';
      
      // Step 1: Network Settings
      if (this.currentStep === 1) {
        if (!this.formData.network_name || this.formData.network_name.trim() === '') {
          this.errorMessage = 'Network name is required';
          return false;
        }
        if (!this.formData.network_subnet || this.formData.network_subnet.trim() === '') {
          this.errorMessage = 'Network subnet (CIDR) is required';
          return false;
        }
        // Validate CIDR format
        this.validateNetworkSubnet();
        if (!this.networkSubnetValid) {
          this.errorMessage = 'Network subnet must be a valid CIDR format (e.g., 10.0.34.0/24)';
          return false;
        }
      }
      
      // Step 2: Web Server Settings
      if (this.currentStep === 2) {
        if (!this.formData.agent_web_address || this.formData.agent_web_address.trim() === '') {
          this.errorMessage = 'Web server address is required';
          return false;
        }
        // At least one of HTTP or HTTPS must be enabled
        if (!this.formData.agent_web_http_enabled && !this.formData.agent_web_https_enabled) {
          this.errorMessage = 'At least one of HTTP or HTTPS must be enabled';
          return false;
        }
        if (this.formData.agent_web_http_enabled && (!this.formData.agent_web_http_port || this.formData.agent_web_http_port <= 0)) {
          this.errorMessage = 'HTTP port is required when HTTP is enabled';
          return false;
        }
        if (this.formData.agent_web_https_enabled) {
          if (!this.formData.agent_web_https_port || this.formData.agent_web_https_port <= 0) {
            this.errorMessage = 'HTTPS port is required when HTTPS is enabled';
            return false;
          }
          if (!this.formData.agent_web_https_tls_cert || this.formData.agent_web_https_tls_cert.trim() === '') {
            this.errorMessage = 'HTTPS TLS certificate path is required when HTTPS is enabled';
            return false;
          }
          if (!this.formData.agent_web_https_tls_key || this.formData.agent_web_https_tls_key.trim() === '') {
            this.errorMessage = 'HTTPS TLS key path is required when HTTPS is enabled';
            return false;
          }
        }
        if (this.formData.agent_web_password_enabled && (!this.formData.agent_web_password_value || this.formData.agent_web_password_value.trim() === '')) {
          this.errorMessage = 'Password is required when password authentication is enabled';
          return false;
        }
      }
      
      // Step 3: VPN & Firewall Settings
      if (this.currentStep === 3) {
        if (this.formData.agent_vpn_enabled && (!this.formData.agent_vpn_port || this.formData.agent_vpn_port <= 0)) {
          this.errorMessage = 'VPN port is required when VPN server is enabled';
          return false;
        }
        if (this.formData.agent_firewall_enabled) {
          if (!this.formData.agent_firewall_utility || this.formData.agent_firewall_utility.trim() === '') {
            this.errorMessage = 'Firewall utility is required when firewall is enabled';
            return false;
          }
          if (!this.formData.agent_firewall_gateway || this.formData.agent_firewall_gateway.trim() === '') {
            this.errorMessage = 'Firewall gateway (interface name) is required when firewall is enabled';
            return false;
          }
        }
      }
      
      // Step 4: Agent Peer Settings
      if (this.currentStep === 4) {
        if (!this.formData.agent_peer_name || this.formData.agent_peer_name.trim() === '') {
          this.errorMessage = 'Agent peer name is required';
          return false;
        }
        if (!this.formData.agent_peer_vpn_internal_address || this.formData.agent_peer_vpn_internal_address.trim() === '') {
          this.errorMessage = 'Agent peer VPN internal address is required';
          return false;
        }
        // Validate that VPN address is within the network subnet CIDR
        this.validateVpnAddress();
        if (!this.vpnAddressValid) {
          this.errorMessage = 'Internal VPN Address must be within the Network Subnet (CIDR) range';
          return false;
        }
        if (!this.formData.agent_peer_vpn_endpoint || this.formData.agent_peer_vpn_endpoint.trim() === '') {
          this.errorMessage = 'Agent peer VPN endpoint is required';
          return false;
        }
        if (!this.formData.agent_peer_kind || this.formData.agent_peer_kind.trim() === '') {
          this.errorMessage = 'Agent peer kind is required';
          return false;
        }
        if (this.formData.agent_peer_icon_enabled && (!this.formData.agent_peer_icon_src || this.formData.agent_peer_icon_src.trim() === '')) {
          this.errorMessage = 'Peer icon source is required when peer icon is enabled';
          return false;
        }
        if (this.formData.agent_peer_dns_enabled) {
          const validDnsAddresses = this.formData.agent_peer_dns_addresses.filter(addr => addr && addr.trim() !== '');
          if (validDnsAddresses.length === 0) {
            this.errorMessage = 'At least one DNS address is required when DNS is enabled';
            return false;
          }
        }
        if (this.formData.agent_peer_mtu_enabled && (!this.formData.agent_peer_mtu_value || this.formData.agent_peer_mtu_value <= 0)) {
          this.errorMessage = 'MTU value is required when MTU is enabled';
          return false;
        }
      }
      
      // Step 5: Default Peer/Connection Settings
      if (this.currentStep === 5) {
        if (!this.formData.default_peer_kind || this.formData.default_peer_kind.trim() === '') {
          this.errorMessage = 'Default peer kind is required';
          return false;
        }
        if (this.formData.default_peer_icon_enabled && (!this.formData.default_peer_icon_src || this.formData.default_peer_icon_src.trim() === '')) {
          this.errorMessage = 'Default peer icon source is required when default peer icon is enabled';
          return false;
        }
        if (this.formData.default_peer_dns_enabled) {
          const validDnsAddresses = this.formData.default_peer_dns_addresses.filter(addr => addr && addr.trim() !== '');
          if (validDnsAddresses.length === 0) {
            this.errorMessage = 'At least one default DNS address is required when default DNS is enabled';
            return false;
          }
        }
        if (this.formData.default_peer_mtu_enabled && (!this.formData.default_peer_mtu_value || this.formData.default_peer_mtu_value <= 0)) {
          this.errorMessage = 'Default MTU value is required when default MTU is enabled';
          return false;
        }
        if (this.formData.default_connection_persistent_keepalive_enabled && (!this.formData.default_connection_persistent_keepalive_period || this.formData.default_connection_persistent_keepalive_period <= 0)) {
          this.errorMessage = 'PersistentKeepalive period is required when PersistentKeepalive is enabled';
          return false;
        }
      }
      
      return true;
    },
    handleNext() {
      if (this.validateStep()) {
        this.currentStep++;
        this.errorMessage = '';
      }
    },
    async submitInit() {
      this.isSubmitting = true;
      this.errorMessage = '';
      
      try {
        // Prepare data in the format expected by the backend
        // When no_prompt is true, ALL fields must be provided (even if None)
        const submitData = {
          network_name: this.formData.network_name || null,
          network_subnet: this.formData.network_subnet || null,
          agent_web_address: this.formData.agent_web_address || null,
          agent_web_http_enabled: this.formData.agent_web_http_enabled,
          agent_web_http_port: this.formData.agent_web_http_enabled ? (this.formData.agent_web_http_port || 80) : 80,
          agent_web_https_enabled: this.formData.agent_web_https_enabled,
          agent_web_https_port: this.formData.agent_web_https_enabled ? (this.formData.agent_web_https_port || 443) : 443,
          agent_web_https_tls_cert: this.formData.agent_web_https_enabled ? (this.formData.agent_web_https_tls_cert || null) : null,
          agent_web_https_tls_key: this.formData.agent_web_https_enabled ? (this.formData.agent_web_https_tls_key || null) : null,
          agent_web_password_enabled: this.formData.agent_web_password_enabled,
          agent_web_password: this.formData.agent_web_password_enabled ? (this.formData.agent_web_password_value || null) : null,
          agent_vpn_enabled: this.formData.agent_vpn_enabled,
          agent_vpn_port: this.formData.agent_vpn_enabled ? (this.formData.agent_vpn_port || 51820) : 51820,
          agent_firewall_enabled: this.formData.agent_firewall_enabled,
          agent_firewall_utility: this.formData.agent_firewall_enabled ? (this.formData.agent_firewall_utility || '/usr/sbin/iptables') : null,
          agent_firewall_gateway: this.formData.agent_firewall_enabled ? (this.formData.agent_firewall_gateway || null) : null,
          agent_peer_name: this.formData.agent_peer_name || null,
          agent_peer_vpn_internal_address: this.formData.agent_peer_vpn_internal_address || null,
          agent_peer_vpn_endpoint: this.formData.agent_peer_vpn_endpoint || null,
          agent_peer_kind: this.formData.agent_peer_kind || null,
          agent_peer_icon_enabled: this.formData.agent_peer_icon_enabled,
          agent_peer_icon_src: this.formData.agent_peer_icon_enabled ? (this.formData.agent_peer_icon_src || null) : null,
          agent_peer_dns_enabled: this.formData.agent_peer_dns_enabled,
          agent_peer_dns_addresses: this.formData.agent_peer_dns_enabled && this.formData.agent_peer_dns_addresses.length > 0 
            ? this.formData.agent_peer_dns_addresses.filter(addr => addr.trim() !== '') 
            : [],
          agent_peer_mtu_enabled: this.formData.agent_peer_mtu_enabled,
          agent_peer_mtu_value: this.formData.agent_peer_mtu_enabled ? (this.formData.agent_peer_mtu_value || 1420) : 1420,
          default_peer_kind: this.formData.default_peer_kind || null,
          default_peer_icon_enabled: this.formData.default_peer_icon_enabled,
          default_peer_icon_src: this.formData.default_peer_icon_enabled ? (this.formData.default_peer_icon_src || null) : null,
          default_peer_dns_enabled: this.formData.default_peer_dns_enabled,
          default_peer_dns_addresses: this.formData.default_peer_dns_enabled && this.formData.default_peer_dns_addresses.length > 0 
            ? this.formData.default_peer_dns_addresses.filter(addr => addr.trim() !== '') 
            : [],
          default_peer_mtu_enabled: this.formData.default_peer_mtu_enabled,
          default_peer_mtu_value: this.formData.default_peer_mtu_enabled ? (this.formData.default_peer_mtu_value || 1420) : 1420,
          default_connection_persistent_keepalive_enabled: this.formData.default_connection_persistent_keepalive_enabled,
          default_connection_persistent_keepalive_period: this.formData.default_connection_persistent_keepalive_enabled 
            ? (this.formData.default_connection_persistent_keepalive_period || 25) 
            : 25,
        };
        
        // Validate required fields
        if (!submitData.network_name) {
          this.errorMessage = 'Network name is required';
          this.isSubmitting = false;
          return;
        }
        if (!submitData.network_subnet) {
          this.errorMessage = 'Network subnet is required';
          this.isSubmitting = false;
          return;
        }
        if (!submitData.agent_web_address) {
          this.errorMessage = 'Web server address is required';
          this.isSubmitting = false;
          return;
        }
        // At least one of HTTP or HTTPS must be enabled
        if (!submitData.agent_web_http_enabled && !submitData.agent_web_https_enabled) {
          this.errorMessage = 'At least one of HTTP or HTTPS must be enabled';
          this.isSubmitting = false;
          return;
        }
        if (submitData.agent_web_https_enabled) {
          if (!submitData.agent_web_https_tls_cert) {
            this.errorMessage = 'HTTPS TLS certificate path is required when HTTPS is enabled';
            this.isSubmitting = false;
            return;
          }
          if (!submitData.agent_web_https_tls_key) {
            this.errorMessage = 'HTTPS TLS key path is required when HTTPS is enabled';
            this.isSubmitting = false;
            return;
          }
        }
        if (submitData.agent_web_password_enabled && !submitData.agent_web_password) {
          this.errorMessage = 'Password is required when password authentication is enabled';
          this.isSubmitting = false;
          return;
        }
        if (submitData.agent_firewall_enabled) {
          if (!submitData.agent_firewall_utility) {
            this.errorMessage = 'Firewall utility is required when firewall is enabled';
            this.isSubmitting = false;
            return;
          }
          if (!submitData.agent_firewall_gateway) {
            this.errorMessage = 'Firewall gateway is required when firewall is enabled';
            this.isSubmitting = false;
            return;
          }
        }
        if (!submitData.agent_peer_name) {
          this.errorMessage = 'Agent peer name is required';
          this.isSubmitting = false;
          return;
        }
        if (!submitData.agent_peer_vpn_internal_address) {
          this.errorMessage = 'Agent peer VPN internal address is required';
          this.isSubmitting = false;
          return;
        }
        if (!submitData.agent_peer_vpn_endpoint) {
          this.errorMessage = 'Agent peer VPN endpoint is required';
          this.isSubmitting = false;
          return;
        }
        if (!submitData.agent_peer_kind) {
          this.errorMessage = 'Agent peer kind is required';
          this.isSubmitting = false;
          return;
        }
        if (submitData.agent_peer_icon_enabled && !submitData.agent_peer_icon_src) {
          this.errorMessage = 'Peer icon source is required when peer icon is enabled';
          this.isSubmitting = false;
          return;
        }
        if (submitData.agent_peer_dns_enabled && submitData.agent_peer_dns_addresses.length === 0) {
          this.errorMessage = 'At least one DNS address is required when DNS is enabled';
          this.isSubmitting = false;
          return;
        }
        if (submitData.default_peer_icon_enabled && !submitData.default_peer_icon_src) {
          this.errorMessage = 'Default peer icon source is required when default peer icon is enabled';
          this.isSubmitting = false;
          return;
        }
        if (submitData.default_peer_dns_enabled && submitData.default_peer_dns_addresses.length === 0) {
          this.errorMessage = 'At least one default DNS address is required when default DNS is enabled';
          this.isSubmitting = false;
          return;
        }
        
        await this.api.post_init(submitData);
        this.$emit('complete');
      } catch (err) {
        const errorMsg = err.message || err.toString() || 'Unknown error';
        this.errorMessage = 'Initialization failed: ' + errorMsg;
        console.error('Init failed:', err);
      } finally {
        this.isSubmitting = false;
      }
    }
  }
}
</script>

<style scoped>
</style>
