<template>
  <div class="bg-card rounded-lg shadow-sm border border-divider p-5 mb-6 hover:shadow-md transition-shadow duration-200">
    <h2 class="text-lg font-semibold text-primary mb-4 flex items-center gap-2">
      <TrendingUp :size="20" class="text-icon" />
      Traffic Analysis
    </h2>
    <div class="h-[200px]">
      <traffic-graph :network="network" :telemetry="telemetry"></traffic-graph>
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
    }
  }
}
</script>
