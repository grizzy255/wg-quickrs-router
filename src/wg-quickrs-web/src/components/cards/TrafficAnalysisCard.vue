<template>
  <div class="bg-card rounded-lg shadow-sm border border-divider p-5 mb-6 hover:shadow-md transition-shadow duration-200">
    <h2 class="text-lg font-semibold text-primary mb-4 flex items-center gap-2">
      <TrendingUp :size="20" class="text-icon" />
      Traffic Analysis
      <span v-if="activeGateway && gatewayName" class="text-sm font-normal text-secondary">
        — {{ gatewayName }}
      </span>
    </h2>
    <div :class="activeGateway ? 'h-[250px]' : 'h-[160px]'">
      <traffic-graph :network="network" :telemetry="telemetry" :active-gateway="activeGateway" :health-status="healthStatus"></traffic-graph>
    </div>
  </div>
</template>

<script>
import { TrendingUp } from 'lucide-vue-next';
import TrafficGraph from '@/src/components/traffic-graph.vue';

export default {
  name: 'TrafficAnalysisCard',
  components: {
    TrendingUp,
    TrafficGraph
  },
  props: {
    network: {
      type: Object,
      default: () => ({})
    },
    telemetry: {
      type: Object,
      default: () => ({})
    },
    activeGateway: {
      type: String,
      default: null
    },
    healthStatus: {
      type: Object,
      default: () => ({})
    }
  },
  computed: {
    gatewayName() {
      if (!this.activeGateway || !this.network || !this.network.peers) {
        return null;
      }
      const peer = this.network.peers[this.activeGateway];
      return peer ? peer.name : this.activeGateway.substring(0, 8) + '...';
    }
  }
}
</script>
