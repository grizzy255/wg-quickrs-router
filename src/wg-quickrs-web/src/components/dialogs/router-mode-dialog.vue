<template>
  <custom-dialog
      :left-button-click="handleCancel"
      modal-classes="max-w-2xl"
      :left-button-text="'Cancel'"
      :right-button-color="'blue'"
      :right-button-click="handleConfirm"
      :right-button-text="'Switch to Router Mode'"
      :right-button-disabled="!cidrValid || routerModeLanCidrInput.trim() === ''"
      icon="warning"
      class="z-10">
    <h3 class="text-lg leading-6 font-medium text-primary mb-3">
      Switch to Router Mode
    </h3>
    <div class="mt-2 text-sm text-muted">
      <p class="mb-3 text-primary">This mode is suitable for routers and gateways that need to forward traffic.</p>

      <p class="mb-3">Switching to Router Mode will:</p>
      <ul class="list-disc list-inside mb-3 space-y-1 text-left">
        <li>Enable packet forwarding (net.ipv4.ip_forward=1)</li>
        <li>Handle overlapping routes using separate routing tables</li>
        <li>Support manual active/backup failover for multiple default routes</li>
        <li>Install advertised 0.0.0.0/0 from peers in seprate routing tables</li>
        <li>Configure firewall rules for NAT/MASQUERADE and forwarding</li>
      </ul>

      <!-- LAN Subnet (CIDR) Input -->
      <div class="mt-4">
        <label class="block text-sm font-medium text-primary mb-2">
          LAN Subnet(s) <span class="text-muted text-xs">(comma-separated, e.g., 192.168.1.0/24, 10.0.0.0/8)</span>
          <span class="text-error">*</span>
        </label>
        <input
            v-model="routerModeLanCidrInput"
            :class="[
              'rounded pl-2 pt-1 pb-1 my-0.5 focus:outline-none focus:ring-2 border-1 outline-none w-full text-sm text-primary bg-input',
              cidrValid ? 'border-input focus:border-blue-500 focus:ring-blue-500' : 'border-input-error focus:border-red-500 focus:ring-red-500'
            ]"
            type="text"
            placeholder="192.168.1.0/24, 10.0.0.0/8"
            @input="validateCidr"
            @keyup.enter="handleConfirm"/>
        <p v-if="!cidrValid && routerModeLanCidrInput" class="text-xs text-error mt-1">
          {{ cidrErrorMessage }}
        </p>
        <p v-else-if="routerModeLanCidrInput.trim() === ''" class="text-xs text-muted mt-1">
          Required: This is used to keep local LAN traffic local while routing internet traffic through the tunnel.
        </p>
        <p v-else class="text-xs text-muted mt-1">
          Multiple LAN segments can be specified separated by commas.
        </p>
      </div>
    </div>
  </custom-dialog>
</template>

<script>
import CustomDialog from "@/src/components/dialogs/custom-dialog.vue";

export default {
  name: "router-mode-dialog",
  components: { CustomDialog },
  props: {
    routerModeLanCidr: {
      type: String,
      default: null
    }
  },
  emits: ['confirm', 'cancel', 'update:routerModeLanCidr'],
  data() {
    return {
      routerModeLanCidrInput: this.routerModeLanCidr || '',
      cidrValid: true,
      cidrErrorMessage: ''
    }
  },
  watch: {
    routerModeLanCidr(newVal) {
      this.routerModeLanCidrInput = newVal || '';
      this.validateCidr();
    }
  },
  methods: {
    validateCidr() {
      if (!this.routerModeLanCidrInput || this.routerModeLanCidrInput.trim() === '') {
        this.cidrValid = false;
        this.cidrErrorMessage = 'At least one CIDR is required';
        return;
      }
      
      // CIDR validation regex: IP address followed by / and subnet mask (0-32)
      const cidrRegex = /^((25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\/([0-9]|[1-2][0-9]|3[0-2])$/;
      
      // Split by comma and validate each CIDR
      const cidrs = this.routerModeLanCidrInput.split(',').map(c => c.trim()).filter(c => c);
      
      if (cidrs.length === 0) {
        this.cidrValid = false;
        this.cidrErrorMessage = 'At least one CIDR is required';
        return;
      }
      
      for (const cidr of cidrs) {
        if (!cidrRegex.test(cidr)) {
          this.cidrValid = false;
          this.cidrErrorMessage = `Invalid CIDR format: "${cidr}". Please use format like 192.168.1.0/24`;
          return;
        }
        
        // Additional validation: check if subnet mask is reasonable
        const parts = cidr.split('/');
        const subnet = parseInt(parts[1]);
        if (subnet === 0 || subnet > 32) {
          this.cidrValid = false;
          this.cidrErrorMessage = `Invalid subnet mask in "${cidr}". Must be between 1 and 32.`;
          return;
        }
      }
      
      this.cidrValid = true;
      this.cidrErrorMessage = '';
    },
    handleConfirm() {
      if (this.cidrValid && this.routerModeLanCidrInput.trim() !== '') {
        // Normalize the CIDRs: trim each, join with ", "
        const cidrs = this.routerModeLanCidrInput.split(',').map(c => c.trim()).filter(c => c);
        const normalizedCidr = cidrs.join(', ');
        this.$emit('confirm', normalizedCidr);
      }
    },
    handleCancel() {
      this.$emit('cancel');
    }
  }
}
</script>

<style scoped>
</style>

