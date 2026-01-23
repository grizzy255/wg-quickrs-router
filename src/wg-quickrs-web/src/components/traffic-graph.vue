<template>
  <div class="w-full h-full flex flex-col gap-1">
    <!-- Main Traffic Graph -->
    <div class="relative flex-1 min-h-0">
      <canvas ref="trafficCanvas" class="w-full h-full"></canvas>
      
      <!-- Tx/Rx labels overlay -->
      <div class="absolute top-1 left-12 flex flex-col items-start space-y-0 text-xs bg-card/60 px-1.5 py-1 rounded backdrop-blur-sm">
        <div class="font-semibold whitespace-pre-wrap text-blue-600/70 dark:text-blue-400/90">
          Tx: {{ tx_avg }} <span class="text-blue-400/50 dark:text-blue-500/70">(peak {{ telem_span }}: {{ tx_peak }})</span>
        </div>
        <div class="font-semibold whitespace-pre-wrap text-green-600/70 dark:text-green-400/90">
          Rx: {{ rx_avg }} <span class="text-green-400/50 dark:text-green-500/70">(peak {{ telem_span }}: {{ rx_peak }})</span>
        </div>
      </div>
    </div>

    <!-- Latency/Packet Loss Mini Graph (only shown when activeGateway is set) -->
    <div v-if="activeGateway" class="relative h-[80px] border-t border-divider/50">
      <canvas ref="latencyCanvas" class="w-full h-full"></canvas>
      
      <!-- Latency/Loss labels overlay -->
      <div class="absolute top-0.5 left-12 flex flex-row items-center gap-4 text-xs bg-card/60 px-1.5 py-0.5 rounded backdrop-blur-sm">
        <div class="font-semibold whitespace-nowrap text-orange-600/90 dark:text-orange-400/90">
          Latency: {{ currentLatency !== null ? currentLatency + 'ms' : '?' }}
          <span v-if="peakLatency !== null" class="text-orange-400/50 dark:text-orange-500/70">(peak: {{ peakLatency }}ms)</span>
        </div>
        <div class="font-semibold whitespace-nowrap" :class="packetLossClass">
          Loss: {{ currentPacketLoss !== null ? currentPacketLoss.toFixed(2) + '%' : '?' }}
        </div>
      </div>
    </div>
  </div>
</template>

<script>
import { SmoothieChart, TimeSeries } from 'smoothie';

function formatThroughputBits(bytesPerSec) {
  if (!isFinite(bytesPerSec) || bytesPerSec <= 0) return "0 b/s";

  const factor = 1000;
  let value = bytesPerSec * 8;
  const units = ["b/s", "kb/s", "Mb/s", "Gb/s", "Tb/s"];

  let unitIndex = 0;
  while (value >= factor && unitIndex < units.length - 1) {
    value /= factor;
    unitIndex++;
  }

  const rounded = Math.round(value * 10) / 10;
  const text = rounded % 1 === 0 ? String(rounded) : rounded.toFixed(1);
  return `${text} ${units[unitIndex]}`;
}

function formatYAxisBits(bytesPerSec) {
  if (!isFinite(bytesPerSec) || bytesPerSec <= 0) return "0";

  const factor = 1000;
  let value = bytesPerSec * 8;
  const units = ["", "K", "M", "G", "T"];

  let unitIndex = 0;
  while (value >= factor && unitIndex < units.length - 1) {
    value /= factor;
    unitIndex++;
  }

  if (value >= 100) {
    return `${Math.round(value)} ${units[unitIndex]}`;
  } else if (value >= 10) {
    return `${value.toFixed(1)} ${units[unitIndex]}`;
  } else {
    return `${value.toFixed(2)} ${units[unitIndex]}`;
  }
}

export default {
  name: "traffic-graph",
  computed: {
    packetLossClass() {
      if (this.currentPacketLoss === null || this.currentPacketLoss === undefined) {
        return 'text-gray-500/70 dark:text-gray-400/70';
      }
      if (this.currentPacketLoss > 5) {
        return 'text-red-600/90 dark:text-red-400/90';
      }
      if (this.currentPacketLoss > 1) {
        return 'text-yellow-600/90 dark:text-yellow-400/90';
      }
      return 'text-green-600/70 dark:text-green-400/70';
    }
  },
  props: {
    network: {
      type: Object,
      default: () => ({}),
    },
    telemetry: {
      type: Object,
      default: () => ({}),
    },
    activeGateway: {
      type: String,
      default: null,
    },
    healthStatus: {
      type: Object,
      default: () => ({}),
    },
  },
  data() {
    return {
      tx_avg: "? b/s",
      rx_avg: "? b/s",
      tx_peak: "? b/s",
      rx_peak: "? b/s",
      telem_span: "?",
      // Traffic chart
      trafficChart: null,
      txSeries: null,
      rxSeries: null,
      lastTimestamp: 0,
      // Latency chart
      latencyChart: null,
      latencySeries: null,
      lastHealthTimestamp: 0,
      // Display values
      currentLatency: null,
      currentPacketLoss: null,
      peakLatency: null,
      latencyHistory: [],
      // Dark mode
      darkModeObserver: null,
    }
  },
  mounted() {
    this.initTrafficChart();
    if (this.activeGateway) {
      this.$nextTick(() => this.initLatencyChart());
    }
    
    // Watch for dark mode changes
    this.darkModeObserver = new MutationObserver(() => {
      this.updateChartColors();
    });
    this.darkModeObserver.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ['class']
    });
  },
  beforeUnmount() {
    if (this.trafficChart) this.trafficChart.stop();
    if (this.latencyChart) this.latencyChart.stop();
    if (this.darkModeObserver) this.darkModeObserver.disconnect();
  },
  watch: {
    activeGateway: {
      handler(newGateway, oldGateway) {
        if (newGateway !== oldGateway) {
          // Clear traffic graph
          if (this.txSeries) this.txSeries.clear();
          if (this.rxSeries) this.rxSeries.clear();
          this.lastTimestamp = 0;
          
          // Clear latency graph
          if (this.latencySeries) this.latencySeries.clear();
          this.lastHealthTimestamp = 0;
          this.latencyHistory = [];
          
          this.resetText();
          
          // Initialize latency chart if gateway is now set
          if (newGateway && !this.latencyChart) {
            this.$nextTick(() => this.initLatencyChart());
          }
        }
      }
    },
    healthStatus: {
      handler(newHealthStatus) {
        if (!this.activeGateway || !newHealthStatus || !newHealthStatus[this.activeGateway]) {
          return;
        }
        
        const health = newHealthStatus[this.activeGateway];
        const now = Date.now();
        
        // Update current values
        this.currentLatency = health.latency_ms;
        this.currentPacketLoss = health.packet_loss_percent;
        
        // Track peak latency and add to series
        if (health.latency_ms !== null && health.latency_ms !== undefined) {
          this.latencyHistory.push(health.latency_ms);
          if (this.latencyHistory.length > 60) {
            this.latencyHistory.shift();
          }
          this.peakLatency = Math.max(...this.latencyHistory);
          
          if (now > this.lastHealthTimestamp && this.latencySeries) {
            this.latencySeries.append(now, health.latency_ms);
            this.lastHealthTimestamp = now;
          }
        }
        
        // Packet loss is displayed as text only (not graphed) since mixing % with ms scale doesn't work
      },
      deep: true
    },
    telemetry: {
      handler() {
        if (this.telemetry === null || !this.telemetry.data || Object.keys(this.telemetry.data).length < 2) {
          this.resetText();
          return;
        }

        const txs = [];
        const rxs = [];
        const timestamps = [];
        let prev_telem_data = {};

        for (const telem_data of this.telemetry.data) {
          if (Object.keys(prev_telem_data).length === 0) {
            prev_telem_data = telem_data;
            continue;
          }

          let tx = 0, rx = 0;
          const ts = (telem_data.timestamp - prev_telem_data.timestamp) / 1000;

          for (const [connection_id, telemetry_details] of Object.entries(telem_data.datum)) {
            if (!prev_telem_data.datum[connection_id]) continue;

            if (this.activeGateway) {
              const isGatewayConnection = connection_id.includes(this.activeGateway);
              if (!isGatewayConnection) continue;
            }

            if (connection_id.startsWith(this.network.this_peer)) {
              tx += telemetry_details.transfer_a_to_b - prev_telem_data.datum[connection_id].transfer_a_to_b;
              rx += telemetry_details.transfer_b_to_a - prev_telem_data.datum[connection_id].transfer_b_to_a;
            } else if (connection_id.endsWith(this.network.this_peer)) {
              tx += telemetry_details.transfer_b_to_a - prev_telem_data.datum[connection_id].transfer_b_to_a;
              rx += telemetry_details.transfer_a_to_b - prev_telem_data.datum[connection_id].transfer_a_to_b;
            }
          }

          txs.push(tx / ts);
          rxs.push(rx / ts);
          timestamps.push(telem_data.timestamp);
          prev_telem_data = telem_data;
        }

        // Update text labels
        const tx_avg = formatThroughputBits(txs.at(-1));
        const rx_avg = formatThroughputBits(rxs.at(-1));
        const maxAvgLen = Math.max(tx_avg.length, rx_avg.length);
        this.tx_avg = tx_avg.padStart(maxAvgLen, " ");
        this.rx_avg = rx_avg.padStart(maxAvgLen, " ");

        const tx_peak_n = Math.max(...txs);
        const rx_peak_n = Math.max(...rxs);
        const tx_peak = formatThroughputBits(tx_peak_n);
        const rx_peak = formatThroughputBits(rx_peak_n);
        const maxPeakLen = Math.max(tx_peak.length, rx_peak.length);
        this.tx_peak = tx_peak.padStart(maxPeakLen, " ");
        this.rx_peak = rx_peak.padStart(maxPeakLen, " ");
        this.telem_span = `${Math.round((this.telemetry.data.at(-1).timestamp - this.telemetry.data[0].timestamp) / 1000)}s`;

        const latestTimestamp = timestamps.at(-1);
        const latestTx = txs.at(-1) || 0;
        const latestRx = rxs.at(-1) || 0;

        if (latestTimestamp > this.lastTimestamp) {
          this.txSeries.append(latestTimestamp, latestTx);
          this.rxSeries.append(latestTimestamp, latestRx);
          this.lastTimestamp = latestTimestamp;
        }
      },
      deep: true
    }
  },
  methods: {
    initTrafficChart() {
      this.txSeries = new TimeSeries();
      this.rxSeries = new TimeSeries();

      const isDarkMode = document.documentElement.classList.contains('dark');
      
      this.trafficChart = new SmoothieChart({
        responsive: true,
        millisPerPixel: 50,
        minValue: 0,
        maxValueScale: 1.2,
        scaleSmoothing: 0.3,
        grid: {
          fillStyle: 'transparent',
          strokeStyle: isDarkMode ? 'rgba(255, 255, 255, 0.1)' : 'rgba(0,0,0,0.08)',
          verticalSections: 4,
          millisPerLine: 10000,
          sharpLines: true,
          borderVisible: false,
        },
        labels: {
          fillStyle: isDarkMode ? 'rgba(209, 213, 219, 0.9)' : 'rgba(80, 80, 80, 0.9)',
          fontSize: 10,
          precision: 0,
          showIntermediateLabels: true,
        },
        yMaxFormatter: (max) => formatYAxisBits(max),
        yMinFormatter: (min) => formatYAxisBits(min),
        yIntermediateFormatter: (val) => formatYAxisBits(val),
        timestampFormatter: (date) => {
          const hours = date.getHours().toString().padStart(2, '0');
          const mins = date.getMinutes().toString().padStart(2, '0');
          const secs = date.getSeconds().toString().padStart(2, '0');
          return `┬ ${hours}:${mins}:${secs}`;
        },
        tooltip: true,
        tooltipLine: {
          lineWidth: 1,
          strokeStyle: isDarkMode ? 'rgba(255, 255, 255, 0.5)' : 'rgba(0, 0, 0, 0.3)',
        },
        interpolation: 'bezier',
      });

      this.trafficChart.addTimeSeries(this.rxSeries, {
        strokeStyle: 'rgba(34, 197, 94, 0.8)',
        fillStyle: 'rgba(34, 197, 94, 0.2)',
        lineWidth: 2,
      });

      this.trafficChart.addTimeSeries(this.txSeries, {
        strokeStyle: 'rgba(59, 130, 246, 0.8)',
        fillStyle: 'rgba(59, 130, 246, 0.2)',
        lineWidth: 2,
      });

      this.trafficChart.streamTo(this.$refs.trafficCanvas, 1000);
    },
    initLatencyChart() {
      if (!this.$refs.latencyCanvas) return;
      
      this.latencySeries = new TimeSeries();

      const isDarkMode = document.documentElement.classList.contains('dark');
      
      this.latencyChart = new SmoothieChart({
        responsive: true,
        millisPerPixel: 50,
        minValue: 0,
        maxValueScale: 1.3,
        scaleSmoothing: 0.3,
        grid: {
          fillStyle: 'transparent',
          strokeStyle: isDarkMode ? 'rgba(255, 255, 255, 0.08)' : 'rgba(0,0,0,0.05)',
          verticalSections: 3,
          millisPerLine: 10000,
          sharpLines: true,
          borderVisible: false,
        },
        labels: {
          fillStyle: isDarkMode ? 'rgba(209, 213, 219, 0.7)' : 'rgba(80, 80, 80, 0.7)',
          fontSize: 9,
          precision: 0,
          showIntermediateLabels: true,
        },
        yMaxFormatter: (max) => `${Math.round(max)}ms`,
        yMinFormatter: (min) => `${Math.round(min)}ms`,
        yIntermediateFormatter: (val) => `${Math.round(val)}ms`,
        timestampFormatter: (date) => {
          const hours = date.getHours().toString().padStart(2, '0');
          const mins = date.getMinutes().toString().padStart(2, '0');
          const secs = date.getSeconds().toString().padStart(2, '0');
          return `┬ ${hours}:${mins}:${secs}`;
        },
        tooltip: true,
        tooltipLine: {
          lineWidth: 1,
          strokeStyle: isDarkMode ? 'rgba(255, 255, 255, 0.4)' : 'rgba(0, 0, 0, 0.2)',
        },
        interpolation: 'step',  // Step interpolation for bar-like appearance
      });

      // Latency bars (orange)
      this.latencyChart.addTimeSeries(this.latencySeries, {
        strokeStyle: 'rgba(249, 115, 22, 0.9)',
        fillStyle: 'rgba(249, 115, 22, 0.4)',
        lineWidth: 2,
      });

      this.latencyChart.streamTo(this.$refs.latencyCanvas, 1000);
    },
    resetText() {
      this.tx_avg = "? b/s";
      this.rx_avg = "? b/s";
      this.tx_peak = "? b/s";
      this.rx_peak = "? b/s";
      this.telem_span = "?";
      this.currentLatency = null;
      this.currentPacketLoss = null;
      this.peakLatency = null;
    },
    updateChartColors() {
      const isDarkMode = document.documentElement.classList.contains('dark');
      
      if (this.trafficChart) {
        this.trafficChart.options.grid.strokeStyle = isDarkMode ? 'rgba(255, 255, 255, 0.1)' : 'rgba(0,0,0,0.08)';
        this.trafficChart.options.labels.fillStyle = isDarkMode ? 'rgba(209, 213, 219, 0.9)' : 'rgba(80, 80, 80, 0.9)';
        if (this.trafficChart.options.tooltipLine) {
          this.trafficChart.options.tooltipLine.strokeStyle = isDarkMode ? 'rgba(255, 255, 255, 0.5)' : 'rgba(0, 0, 0, 0.3)';
        }
        this.trafficChart.render();
      }
      
      if (this.latencyChart) {
        this.latencyChart.options.grid.strokeStyle = isDarkMode ? 'rgba(255, 255, 255, 0.08)' : 'rgba(0,0,0,0.05)';
        this.latencyChart.options.labels.fillStyle = isDarkMode ? 'rgba(209, 213, 219, 0.7)' : 'rgba(80, 80, 80, 0.7)';
        if (this.latencyChart.options.tooltipLine) {
          this.latencyChart.options.tooltipLine.strokeStyle = isDarkMode ? 'rgba(255, 255, 255, 0.4)' : 'rgba(0, 0, 0, 0.2)';
        }
        this.latencyChart.render();
      }
    },
  }
}
</script>
