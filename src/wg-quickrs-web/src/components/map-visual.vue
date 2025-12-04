<template>

  <div id="graph" class="shadow-md rounded-lg bg-card justify-center"></div>

</template>

<script>
import ForceGraph from "force-graph";
import FastEqual from "fast-deep-equal";
import {get_connection_id_wasm} from "@/pkg/wg_quickrs_lib.js";

// SVG path data for icons (from Lucide icons)
const iconPaths = {
  server: 'M2 9h20M2 15h20M6 9v6M10 9v6M14 9v6M18 9v6M4 5h16a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V7a2 2 0 0 1 2-2z',
  desktop: 'M20 3H4a2 2 0 0 0-2 2v10a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V5a2 2 0 0 0-2-2zM8 21h8M12 17v4',
  laptop: 'M20 16V7a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v9m16 0H4m16 0 1.28 2.55a1 1 0 0 1-.9 1.45H3.62a1 1 0 0 1-.9-1.45L4 16',
  tablet: 'M12 18h.01M18 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V4a2 2 0 0 0-2-2z',
  phone: 'M12 18h.01M17 2H7a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V4a2 2 0 0 0-2-2z',
  IoT: 'M12 16a4 4 0 1 0 0-8 4 4 0 0 0 0 8zM12 3v1M12 20v1M3 12h1M20 12h1M5.64 5.64l.7.7M17.66 17.66l.7.7M5.64 18.36l.7-.7M17.66 6.34l.7-.7',
  other: 'M12 22c5.523 0 10-4.477 10-10S17.523 2 12 2 2 6.477 2 12s4.477 10 10 10zM12 16v.01M12 8v4',
  landmark: 'M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5',
};

// Generate SVG data URL with dynamic color
function getIconDataUrl(iconType, color) {
  const path = iconPaths[iconType] || iconPaths.other;
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="${color}" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">${path.split('M').map((p, i) => i === 0 ? '' : `<path d="M${p}"/>`).join('')}</svg>`;
  return 'data:image/svg+xml,' + encodeURIComponent(svg);
}

// Get icon color based on dark mode - uses CSS variable for consistency
function getIconColor() {
  const root = document.documentElement;
  return getComputedStyle(root).getPropertyValue('--text-icon').trim() || (isDarkMode() ? '#6b7280' : '#6b7280');
}

// Light mode colors
const tw_gray_50 = 'oklch(98.5% 0.002 247.839)';
const tw_gray_200 = 'oklch(92.8% 0.006 264.531)';
const tw_gray_300 = 'oklch(87.2% 0.01 258.338)';
const tw_gray_500 = 'oklch(55.1% 0.027 264.364)';
const tw_gray_700 = 'oklch(37.3% 0.034 259.733)';

// Dark mode colors
const tw_gray_800 = 'oklch(27.4% 0.024 264.531)';
const tw_gray_900 = 'oklch(21% 0.024 264.531)';
const tw_gray_100 = 'oklch(96.7% 0.003 264.531)';

// Helper to check dark mode
function isDarkMode() {
  return document.documentElement.classList.contains('dark');
}

// Dynamic color getters - use CSS variables for consistency
function getLabelBgColor() {
  const root = document.documentElement;
  return getComputedStyle(root).getPropertyValue('--bg-card').trim() || (isDarkMode() ? tw_gray_800 : tw_gray_50);
}

function getLabelTextColor() {
  const root = document.documentElement;
  return getComputedStyle(root).getPropertyValue('--text-primary').trim() || (isDarkMode() ? tw_gray_100 : tw_gray_700);
}

function getNodeIconBgColor() {
  const root = document.documentElement;
  return getComputedStyle(root).getPropertyValue('--bg-card').trim() || (isDarkMode() ? tw_gray_800 : tw_gray_50);
}

// Get link colors based on dark mode
function getLinkColor(strength) {
  const root = document.documentElement;
  if (isDarkMode()) {
    // Dark mode: use lighter grays for visibility
    switch (strength) {
      case 1:
        return getComputedStyle(root).getPropertyValue('--border-divider').trim() || tw_gray_700;
      case 2:
        return getComputedStyle(root).getPropertyValue('--border-input').trim() || tw_gray_600;
      case 3:
        return getComputedStyle(root).getPropertyValue('--text-icon').trim() || tw_gray_500;
      default:
        return tw_gray_700;
    }
  } else {
    // Light mode: use existing grays
    switch (strength) {
      case 1:
        return tw_gray_200;
      case 2:
        return tw_gray_300;
      case 3:
        return tw_gray_500;
      default:
        return tw_gray_200;
    }
  }
}
const tw_orange_600 = 'oklch(70.5% 0.213 47.604)';
const tw_emerald_600 = 'oklch(59.6% 0.145 163.225)';
const tw_red_600 = 'oklch(57.7% 0.245 27.325)';
const tw_indigo_600 = 'oklch(51.1% 0.262 276.966)';
const tw_amber_600 = 'oklch(68.1% 0.162 75.834)';
const tw_purple_600 = 'oklch(55.8% 0.288 302.321)';
const tw_pink_600 = 'oklch(59.2% 0.249 0.584)';
const tw_sky_600 = 'oklch(58.8% 0.158 241.966)';
const tw_gray_600 = 'oklch(44.6% 0.03 256.802)';

const nodeHoverHighlightColorMap = {
  "node": tw_orange_600,
  "neighbor": tw_emerald_600,
}

const nodeKindHighlightColorMap = {
  "server": tw_red_600,
  "desktop": tw_indigo_600,
  "laptop": tw_amber_600,
  "tablet": tw_purple_600,
  "phone": tw_pink_600,
  "IoT": tw_sky_600,
  "other": tw_gray_600,
}

const highlightNodes = new Set();
const highlightLinks = new Set();
let hoverNode = null;

function roundRect(ctx, x, y, width, height, radius) {
  ctx.beginPath();
  ctx.moveTo(x + radius, y);
  ctx.lineTo(x + width - radius, y);
  ctx.quadraticCurveTo(x + width, y, x + width, y + radius);
  ctx.lineTo(x + width, y + height - radius);
  ctx.quadraticCurveTo(x + width, y + height, x + width - radius, y + height);
  ctx.lineTo(x + radius, y + height);
  ctx.quadraticCurveTo(x, y + height, x, y + height - radius);
  ctx.lineTo(x, y + radius);
  ctx.quadraticCurveTo(x, y, x + radius, y);
  ctx.closePath();
  ctx.fill();
}

export default {
  name: "map-visual",
  props: {
    network: {
      type: Object,
      default: {},
    },
    telemetry: {
      type: Object,
      default: {},
    },
  },
  data() {
    return {
      initializedGraph: false,
      graph: null,
      container: null,
    }
  },
  mounted() {
    this.container = document.getElementById('graph-app');

    // resize on window change
    window.addEventListener('resize', () => {
      if (this.graph === null) return;
      // 24px = px-3 padding
      this.graph.width(this.container.offsetWidth - 24).height(this.container.offsetHeight);
    });
  },
  watch: {
    network: function (newVal, oldVal) { // watch it
      if (FastEqual(newVal, oldVal)) {
        return;
      }

      if (this.initializedGraph) {
        try {
          this.graph.graphData(this.calculateForceGraphData(newVal));
        } catch {
          // Ignore graph update errors
        }
      }

      if (!this.initializedGraph) {
        try {
          // Enhanced orbital layout: center node in middle, peers in orbit
          const centerNode = this.network?.this_peer;
          
          this.graph = ForceGraph()(document.getElementById('graph'))
              .autoPauseRedraw(false)
              .width(this.container.offsetWidth - 24)
              .height(this.container.offsetHeight)
              .d3Force('center', null)
              .zoomToFit(100, 20)
              .maxZoom(20)
              .nodeId('id')
              .nodeLabel(null)
              .nodeCanvasObject((node, ctx) => {
                if (hoverNode === node) {
                  ctx.beginPath();
                  ctx.arc(node.x, node.y, node.size * 0.65, 0, 2 * Math.PI, false);
                  ctx.fillStyle = nodeHoverHighlightColorMap['node'];
                  ctx.fill();
                } else if (highlightNodes.has(node)) {
                  ctx.beginPath();
                  ctx.arc(node.x, node.y, node.size * 0.65, 0, 2 * Math.PI, false);
                  ctx.fillStyle = nodeHoverHighlightColorMap['neighbor'];
                  ctx.fill();
                }
                ctx.beginPath();
                ctx.arc(node.x, node.y, node.size * 0.55, 0, 2 * Math.PI, false);
                ctx.fillStyle = nodeKindHighlightColorMap[node.kind] || nodeKindHighlightColorMap['other'];
                ctx.fill();

                const iconColor = getIconColor();
                const img = new Image();
                if (node.icon.enabled) {
                  img.src = node.icon.src;
                } else {
                  img.src = getIconDataUrl(node.kind, iconColor);
                }
                const cis = this.getGraphNodeIcon(img, 500);
                ctx.drawImage(cis, node.x - node.size / 2, node.y - node.size / 2, node.size, node.size);
                
                if (node.id === this.network.this_peer) {
                  const marker_img = new Image();
                  marker_img.src = getIconDataUrl('landmark', iconColor);
                  const marker = this.getGraphNodeIcon(marker_img, 500);
                  ctx.drawImage(marker, node.x - node.size / 4, node.y - 3 * node.size / 4, node.size / 2, node.size / 2);
                }

                // node label "text" 1/2
                const fontSize = 2;
                ctx.font = `${fontSize}px monospace`;

                // node label "div"
                const textWidth = ctx.measureText(node.name).width;
                const bckgDimensions = [textWidth, fontSize].map(n => n + fontSize * 0.3); // some padding
                ctx.fillStyle = getLabelBgColor();
                roundRect(ctx,
                    node.x - bckgDimensions[0] / 2.,
                    node.y + node.size / 2. - bckgDimensions[1] / 2.,
                    bckgDimensions[0],
                    bckgDimensions[1],
                    1. // corner radius
                );

                // node label "text" 2/2
                ctx.fillStyle = getLabelTextColor();
                ctx.textAlign = 'center';
                ctx.textBaseline = "alphabetic";
                ctx.fillText(node.name, node.x, node.y + node.size / 2. + fontSize * 0.3);
                node.__bckgDimensions = bckgDimensions;
              })
              .nodePointerAreaPaint((node, color, ctx) => {
                ctx.beginPath();
                ctx.arc(node.x, node.y, node.size * 0.55, 0, 2 * Math.PI, false);
                ctx.fillStyle = color;
                ctx.fill();

                const bckgDimensions = node.__bckgDimensions;
                roundRect(ctx,
                    node.x - bckgDimensions[0] / 2.,
                    node.y + node.size / 2. - bckgDimensions[1] / 2.,
                    bckgDimensions[0],
                    bckgDimensions[1],
                    1 // corner radius
                );
              })
              .onNodeHover(node => {
                highlightNodes.clear();
                highlightLinks.clear();
                if (node) {
                  highlightNodes.add(node);
                  node.neighbors.forEach(neighbor => highlightNodes.add(neighbor));
                  node.links.forEach(link => highlightLinks.add(link));
                }
                hoverNode = node || null;
              })
              .onNodeDrag(node => {
                highlightNodes.clear();
                highlightLinks.clear();
                if (node) {
                  highlightNodes.add(node);
                  node.neighbors.forEach(neighbor => highlightNodes.add(neighbor));
                  node.links.forEach(link => highlightLinks.add(link));
                }
                hoverNode = node || null;
              })
              .onNodeDragEnd(_ => {
                highlightNodes.clear();
                highlightLinks.clear();
                hoverNode = null;
              })
              .onLinkHover(link => {
                highlightNodes.clear();
                highlightLinks.clear();

                if (link) {
                  highlightLinks.add(link);
                  highlightNodes.add(link.source);
                  highlightNodes.add(link.target);
                }
              })
              .linkSource('source')
              .linkTarget('target')
              .linkAutoColorBy('color')
              .linkWidth(link => highlightLinks.has(link) ? 10 : link.strength)
              .linkDirectionalParticleCanvasObject((x, y, link, ctx, _globalScale) => {
                const target = link.target;
                const dx = target.x - x;
                const dy = target.y - y;
                const angle = Math.atan2(dy, dx);

                const size = 1.5;
                const w = size;
                const h = size;

                ctx.save();
                ctx.translate(x, y);
                ctx.rotate(angle + Math.PI / 2); // fix direction mismatch

                // arrow shape
                ctx.beginPath();
                ctx.moveTo(0, -h);                   // Tip
                ctx.lineTo(w * 0.6, h * 0.4);        // Right wing
                ctx.lineTo(0, h * 0.2);              // Tail center
                ctx.lineTo(-w * 0.6, h * 0.4);       // Left wing

                ctx.closePath();

                // Fill and stroke
                ctx.fill();
                ctx.restore();
              })
              .linkDirectionalParticleSpeed(0.025)
              .linkDirectionalParticleColor((particle_info) => {
                // Use consistent colors that work in both light and dark mode
                if (particle_info.source.id === this.network.this_peer) {
                  return 'rgba(59,130,246,0.6)';  // RX - blue, slightly more opaque for dark mode visibility
                } else if (particle_info.target.id === this.network.this_peer) {
                  return 'rgba(34,197,94,0.6)';  // TX - green, slightly more opaque for dark mode visibility
                }
                return particle_info.color;
              })
              .cooldownTicks(10);

          this.graph.onEngineStop(() => this.graph.zoomToFit(400, 20));
          this.graph.onBackgroundClick(() => this.graph.zoomToFit(400, 20));
          this.graph.onNodeClick(node => {
            // Center/zoom on node
            this.graph.centerAt(node.x, node.y, 400);
            this.graph.zoom(20, 400);

            this.$emit('peer-selected', node.id);
          });

          // Add orbital positioning after graph is created
          let graphData = this.calculateForceGraphData(newVal);
          if (centerNode) {
            // Initialize node positions for orbital layout
            const centerX = this.container.offsetWidth / 2;
            const centerY = this.container.offsetHeight / 2;
            const peerCount = Object.keys(this.network?.peers || {}).length;
            const baseRadius = Math.min(250, Math.max(120, peerCount * 35));
            
            let peerIndex = 0;
            graphData.nodes.forEach((node) => {
              if (node.id === centerNode) {
                node.x = centerX;
                node.y = centerY;
                node.fx = centerX; // Fix center node position
                node.fy = centerY;
              } else {
                // Position peers in orbit
                const angle = peerIndex * (2 * Math.PI / Math.max(1, peerCount - 1));
                node.x = centerX + baseRadius * Math.cos(angle);
                node.y = centerY + baseRadius * Math.sin(angle);
                peerIndex++;
              }
            });
          }
          
          this.graph.graphData(graphData);
          this.initializedGraph = true;
        } catch {
          // Ignore graph initialization errors
        }
      }
    },
    telemetry: {
      handler() {
        if (this.graph === null) return;
        if (this.telemetry === null) return;
        if (this.telemetry.data.length < 2) return;
        let last_data = this.telemetry.data[this.telemetry.data.length - 1];
        let previous_data = this.telemetry.data[this.telemetry.data.length - 2];

        for (const [connection_id, telemetry_details] of Object.entries(last_data.datum)) {
          for (const link of this.graph.graphData().links) {
            if (link.source.id === undefined) continue;
            if (connection_id !== get_connection_id_wasm(link.source.id, link.target.id)) continue;
            if (!Object.keys(previous_data.datum).includes(connection_id)) continue;

            const trafficBytesPrev = connection_id.startsWith(link.source.id) ? previous_data.datum[connection_id].transfer_a_to_b : previous_data.datum[connection_id].transfer_b_to_a;
            const trafficBytesCurr = connection_id.startsWith(link.source.id) ? telemetry_details.transfer_a_to_b : telemetry_details.transfer_b_to_a;
            const trafficBytesDiff = trafficBytesCurr - trafficBytesPrev;
            if (trafficBytesDiff === 0) continue;
            const ts = (last_data.timestamp - previous_data.timestamp) / 1000.;
            const trafficMbitsPerSec = trafficBytesDiff / ts * 8 / 1000. / 1000.;

            // 80-100 mbps -> 10 particles
            const particleCount = Math.ceil(Math.min(trafficMbitsPerSec / 10., 10));
            this.graphEmitParticles(link, particleCount).then().catch();
          }
        }
      },
      deep: true
    }
  },
  computed: {},
  methods: {
    getConnectionPeers(connectionId) {
      const ab = connectionId.split('*');
      return {a: ab[0], b: ab[1]};
    },
    calculateForceGraphData(network) {
      const peerSize = {};
      Object.keys(network.peers).forEach(peerId => {
        peerSize[peerId] = 1;
      });
      const forceG = {nodes: [], links: []};
      for (const [peerId, peerDetails] of Object.entries(network.peers)) {
        forceG.nodes.push({
          id: peerId,
          name: peerDetails.name,
          endpoint: peerDetails.endpoint,
          size: Math.sqrt(peerSize[peerId]) * 7,
          kind: peerDetails.kind,
          icon: peerDetails.icon,
          neighbors: [],
          links: [],
          __bckgDimensions: [0, 0]
        });
      }

      for (const [connectionId, connectionDetails] of Object.entries(network.connections)) {
        if (connectionDetails.enabled) {
          const {a, b} = this.getConnectionPeers(connectionId);
          const linkColorStrength = 1
              + network.static_peer_ids.includes(a)
              + network.static_peer_ids.includes(b);
          const color = getLinkColor(linkColorStrength);
          forceG.links.push({
            source: a, target: b, particleCount: 0, color, strength: linkColorStrength,
          });
          forceG.links.push({
            source: b, target: a, particleCount: 0, color, strength: linkColorStrength,
          });
        }

        // cross-link node objects
        forceG.links.forEach(link => {
          const a = forceG.nodes.find(item => item.id === link.source);
          const b = forceG.nodes.find(item => item.id === link.target);
          a.neighbors.push(b);
          b.neighbors.push(a);
          a.links.push(link);
          b.links.push(link);
        });
      }
      return forceG;
    },
    getGraphNodeIcon(image, size) {
      const tmpCanvas = document.createElement('canvas');
      const tmpCtx = tmpCanvas.getContext('2d');

      tmpCanvas.width = size;
      tmpCanvas.height = size;

      // draw the cached images to temporary canvas and return the context
      tmpCtx.save();
      tmpCtx.beginPath();
      tmpCtx.arc(size / 2, size / 2, size / 2, 0, Math.PI * 2, true);
      tmpCtx.closePath();
      tmpCtx.clip();
      tmpCtx.fillStyle = getNodeIconBgColor();
      tmpCtx.fillRect(0, 0, size, size);
      tmpCtx.drawImage(image, size / 4, size / 4, size / 2, size / 2);
      
      return tmpCanvas;
    },
    async graphEmitParticles(link, particleCount) {
      for (let i = 0; i < particleCount; i++) {
        this.graph.emitParticle(link);
        await new Promise(r => setTimeout(r, 1000 / particleCount));
      }
    },
  }
}
</script>

<style scoped>

</style>