<template>
  <div class="w-full h-full relative overflow-hidden">
    <!-- Canvas with left padding for Y-axis labels -->
    <canvas ref="trafficCanvas" class="w-full h-full"></canvas>

    <!-- Tx/Rx labels on the left (peak + current) -->
    <div
        class="absolute top-1 left-12 flex flex-col items-start space-y-0 text-xs bg-card/60 px-1.5 py-1 rounded backdrop-blur-sm">
      <div class="font-semibold whitespace-pre-wrap text-blue-600/70 dark:text-blue-400/90">
        Tx: {{ tx_avg }} <span class="text-blue-400/50 dark:text-blue-500/70">(peak {{ telem_span }}: {{ tx_peak }})</span>
      </div>
      <div class="font-semibold whitespace-pre-wrap text-green-600/70 dark:text-green-400/90">
        Rx: {{ rx_avg }} <span class="text-green-400/50 dark:text-green-500/70">(peak {{ telem_span }}: {{ rx_peak }})</span>
      </div>
    </div>
  </div>
</template>

<script>
import { SmoothieChart, TimeSeries } from 'smoothie';

// Minimum floor: 100 Kbps in Bytes/s
const MIN_Y_MAX = 100 * 1000 / 8; // 12,500 Bytes/s

function formatThroughputBits(bytesPerSec) {
  if (!isFinite(bytesPerSec) || bytesPerSec <= 0) return "0 b/s";

  const factor = 1000; // SI scaling
  let value = bytesPerSec * 8; // convert to bits/s

  const units = ["b/s", "kb/s", "Mb/s", "Gb/s", "Tb/s"];

  let unitIndex = 0;
  while (value >= factor && unitIndex < units.length - 1) {
    value /= factor;
    unitIndex++;
  }

  // round to 1 decimal, drop trailing .0
  const rounded = Math.round(value * 10) / 10;
  const text = rounded % 1 === 0 ? String(rounded) : rounded.toFixed(1);

  return `${text} ${units[unitIndex]}`;
}

// Format Y-axis values like OPNsense: "3.0 M", "500.00 K", etc.
function formatYAxisBits(bytesPerSec) {
  if (!isFinite(bytesPerSec) || bytesPerSec <= 0) return "0";

  const factor = 1000; // SI scaling
  let value = bytesPerSec * 8; // convert to bits/s

  const units = ["", "K", "M", "G", "T"];

  let unitIndex = 0;
  while (value >= factor && unitIndex < units.length - 1) {
    value /= factor;
    unitIndex++;
  }

  // Format with 2 decimal places for cleaner display
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
  props: {
    network: {
      type: Object,
      default: () => ({}),
    },
    telemetry: {
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
      chart: null,
      txSeries: null,
      rxSeries: null,
      lastTimestamp: 0,
      darkModeObserver: null,
    }
  },
  mounted() {
    // Create TimeSeries for TX and RX
    this.txSeries = new TimeSeries();
    this.rxSeries = new TimeSeries();

    // Detect dark mode
    const isDarkMode = document.documentElement.classList.contains('dark');
    
    // Create SmoothieChart with OPNsense-like settings
    this.chart = new SmoothieChart({
      responsive: true,
      millisPerPixel: 50,           // Time density
      minValue: 0,                  // Always start at zero
      maxValueScale: 1.2,           // 20% headroom above max (like OPNsense)
      scaleSmoothing: 0.3,          // Smooth scaling transitions (hysteresis effect)
      grid: {
        fillStyle: 'transparent',
        strokeStyle: isDarkMode ? 'rgba(255, 255, 255, 0.1)' : 'rgba(0,0,0,0.08)',
        verticalSections: 5,        // 5 horizontal grid lines for Y-axis
        millisPerLine: 10000,       // Vertical line every 10 seconds
        sharpLines: true,
        borderVisible: false,
      },
      labels: {
        fillStyle: isDarkMode ? 'rgba(209, 213, 219, 0.9)' : 'rgba(80, 80, 80, 0.9)',
        fontSize: 10,
        precision: 0,
        showIntermediateLabels: true,
      },
      // Format Y-axis labels like OPNsense (bits/s with K, M, G suffixes)
      yMaxFormatter: (max) => formatYAxisBits(max),
      yMinFormatter: (min) => formatYAxisBits(min),
      yIntermediateFormatter: (val) => formatYAxisBits(val),
      timestampFormatter: (date) => {
        // Format like OPNsense: HH:MM:SS with tick mark
        const hours = date.getHours().toString().padStart(2, '0');
        const mins = date.getMinutes().toString().padStart(2, '0');
        const secs = date.getSeconds().toString().padStart(2, '0');
        return `┬ ${hours}:${mins}:${secs}`;
      },
      // Enable tooltip - shows vertical line on hover
      tooltip: true,
      tooltipLine: {
        lineWidth: 1,
        strokeStyle: isDarkMode ? 'rgba(255, 255, 255, 0.5)' : 'rgba(0, 0, 0, 0.3)',
      },
      interpolation: 'bezier',      // Smooth curves
    });

    // Add RX series (green) - drawn first (behind)
    this.chart.addTimeSeries(this.rxSeries, {
      strokeStyle: 'rgba(34, 197, 94, 0.8)',   // emerald-500
      fillStyle: 'rgba(34, 197, 94, 0.2)',
      lineWidth: 2,
    });

    // Add TX series (blue) - drawn second (in front)
    this.chart.addTimeSeries(this.txSeries, {
      strokeStyle: 'rgba(59, 130, 246, 0.8)',  // blue-500
      fillStyle: 'rgba(59, 130, 246, 0.2)',
      lineWidth: 2,
    });

    // Stream to canvas
    this.chart.streamTo(this.$refs.trafficCanvas, 1000); // 1 second delay for smooth scrolling
    
    // Watch for dark mode changes and update chart colors
    this.darkModeObserver = new MutationObserver(() => {
      this.updateChartColors();
    });
    this.darkModeObserver.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ['class']
    });
  },
  beforeUnmount() {
    if (this.chart) {
      this.chart.stop();
    }
    if (this.darkModeObserver) {
      this.darkModeObserver.disconnect();
    }
  },
  beforeUnmount() {
    if (this.chart) {
      this.chart.stop();
    }
  },
  watch: {
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

            if (connection_id.startsWith(this.network.this_peer)) {
              tx += telemetry_details.transfer_a_to_b - prev_telem_data.datum[connection_id].transfer_a_to_b;
              rx += telemetry_details.transfer_b_to_a - prev_telem_data.datum[connection_id].transfer_b_to_a;
            } else if (connection_id.endsWith(this.network.this_peer)) {
              tx += telemetry_details.transfer_b_to_a - prev_telem_data.datum[connection_id].transfer_b_to_a;
              rx += telemetry_details.transfer_a_to_b - prev_telem_data.datum[connection_id].transfer_a_to_b;
            }
          }

          // Rates in Bytes/s
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

        // Add latest data point to Smoothie TimeSeries
        const latestTimestamp = timestamps.at(-1);
        const latestTx = txs.at(-1) || 0;
        const latestRx = rxs.at(-1) || 0;

        // Only add new points (avoid duplicates)
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
    resetText() {
      this.tx_avg = "? b/s";
      this.rx_avg = "? b/s";
      this.tx_peak = "? b/s";
      this.rx_peak = "? b/s";
      this.telem_span = "?";
    },
    updateChartColors() {
      if (!this.chart) return;
      
      const isDarkMode = document.documentElement.classList.contains('dark');
      
      // Update grid colors
      this.chart.options.grid.strokeStyle = isDarkMode ? 'rgba(255, 255, 255, 0.1)' : 'rgba(0,0,0,0.08)';
      
      // Update label colors
      this.chart.options.labels.fillStyle = isDarkMode ? 'rgba(209, 213, 219, 0.9)' : 'rgba(80, 80, 80, 0.9)';
      
      // Update tooltip line color
      if (this.chart.options.tooltipLine) {
        this.chart.options.tooltipLine.strokeStyle = isDarkMode ? 'rgba(255, 255, 255, 0.5)' : 'rgba(0, 0, 0, 0.3)';
      }
      
      // Redraw the chart with new colors
      this.chart.render();
    },
  }
}
</script>
