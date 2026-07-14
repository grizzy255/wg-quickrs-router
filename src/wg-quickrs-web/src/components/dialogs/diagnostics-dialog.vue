<template>
  <div class="fixed inset-0 bg-backdrop z-50 flex items-center justify-center p-4" @click.self="$emit('close')">
    <div class="bg-card rounded-lg shadow-xl w-full max-w-4xl max-h-[90vh] flex flex-col border border-divider">
      <!-- Header -->
      <div class="flex items-center justify-between px-6 py-4 border-b border-divider">
        <div class="flex items-center gap-3">
          <Stethoscope :size="24" class="text-icon" />
          <h2 class="text-xl font-semibold text-primary">Network Diagnostics</h2>
        </div>
        <button @click="$emit('close')" class="text-secondary hover:text-primary transition-colors">
          <X :size="24" />
        </button>
      </div>

      <!-- Tabs -->
      <div class="flex border-b border-divider px-6">
        <button 
          v-for="tab in tabs" 
          :key="tab.id"
          @click="activeTab = tab.id"
          :class="[
            'px-4 py-3 text-sm font-medium transition-colors relative',
            activeTab === tab.id 
              ? 'text-blue-500' 
              : 'text-secondary hover:text-primary'
          ]">
          <div class="flex items-center gap-2">
            <component :is="tab.icon" :size="16" />
            {{ tab.name }}
          </div>
          <div v-if="activeTab === tab.id" class="absolute bottom-0 left-0 right-0 h-0.5 bg-blue-500"></div>
        </button>
      </div>

      <!-- Content -->
      <div class="flex-1 overflow-y-auto p-6">
        <!-- Network Tools Tab -->
        <div v-if="activeTab === 'network'" class="space-y-6">
          <!-- Tool Selection -->
          <div class="flex gap-2">
            <button 
              v-for="tool in networkTools" 
              :key="tool.id"
              @click="selectedTool = tool.id"
              :class="[
                'px-4 py-2 rounded-lg text-sm font-medium transition-colors',
                selectedTool === tool.id 
                  ? 'bg-blue-500 text-white' 
                  : 'bg-button text-button hover:bg-button-hover'
              ]">
              {{ tool.name }}
            </button>
          </div>

          <!-- Route Selection -->
          <div class="flex items-center gap-3 p-3 bg-page rounded-lg border border-divider">
            <span class="text-sm text-secondary">Route via:</span>
            <label class="flex items-center gap-2 cursor-pointer">
              <input type="radio" v-model="selectedInterface" value="" class="w-4 h-4" />
              <span class="text-sm text-primary">Default (LAN)</span>
            </label>
            <label class="flex items-center gap-2 cursor-pointer">
              <input type="radio" v-model="selectedInterface" value="wg" class="w-4 h-4" />
              <span class="text-sm text-primary">WireGuard Tunnel</span>
            </label>
          </div>

          <!-- Ping Tool -->
          <div v-if="selectedTool === 'ping'" class="space-y-4">
            <div class="flex gap-3">
              <input 
                v-model="pingTarget"
                type="text"
                placeholder="Enter hostname or IP (e.g., google.com, 8.8.8.8)"
                class="flex-1 px-4 py-2 bg-input border border-divider rounded-lg text-primary placeholder-secondary focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                @keyup.enter="runPing"
              />
              <select v-model="pingCount" class="px-3 py-2 bg-input border border-divider rounded-lg text-primary">
                <option :value="4">4 pings</option>
                <option :value="8">8 pings</option>
                <option :value="12">12 pings</option>
              </select>
              <button 
                @click="runPing"
                :disabled="loading || !pingTarget"
                class="px-4 py-2 bg-blue-500 text-white rounded-lg hover:bg-blue-600 disabled:bg-gray-500 disabled:cursor-not-allowed flex items-center gap-2">
                <Loader2 v-if="loading" :size="16" class="animate-spin" />
                <Play v-else :size="16" />
                Run
              </button>
            </div>
          </div>

          <!-- Traceroute Tool -->
          <div v-if="selectedTool === 'traceroute'" class="space-y-4">
            <div class="flex gap-3">
              <input 
                v-model="tracerouteTarget"
                type="text"
                placeholder="Enter hostname or IP (e.g., google.com)"
                class="flex-1 px-4 py-2 bg-input border border-divider rounded-lg text-primary placeholder-secondary focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                @keyup.enter="runTraceroute"
              />
              <select v-model="tracerouteHops" class="px-3 py-2 bg-input border border-divider rounded-lg text-primary">
                <option :value="15">15 hops</option>
                <option :value="20">20 hops</option>
                <option :value="30">30 hops</option>
              </select>
              <button 
                @click="runTraceroute"
                :disabled="loading || !tracerouteTarget"
                class="px-4 py-2 bg-blue-500 text-white rounded-lg hover:bg-blue-600 disabled:bg-gray-500 disabled:cursor-not-allowed flex items-center gap-2">
                <Loader2 v-if="loading" :size="16" class="animate-spin" />
                <Play v-else :size="16" />
                Run
              </button>
            </div>
          </div>

          <!-- DNS Lookup Tool -->
          <div v-if="selectedTool === 'dns'" class="space-y-4">
            <div class="flex gap-3 flex-wrap">
              <input 
                v-model="dnsHostname"
                type="text"
                placeholder="Enter hostname (e.g., google.com)"
                class="flex-1 min-w-[200px] px-4 py-2 bg-input border border-divider rounded-lg text-primary placeholder-secondary focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                @keyup.enter="runDns"
              />
              <input 
                v-model="dnsServer"
                type="text"
                placeholder="DNS server (optional, e.g., 8.8.8.8)"
                class="w-48 px-4 py-2 bg-input border border-divider rounded-lg text-primary placeholder-secondary focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              />
              <select v-model="dnsType" class="px-3 py-2 bg-input border border-divider rounded-lg text-primary">
                <option value="">Any</option>
                <option value="A">A</option>
                <option value="AAAA">AAAA</option>
                <option value="MX">MX</option>
                <option value="TXT">TXT</option>
                <option value="NS">NS</option>
                <option value="CNAME">CNAME</option>
              </select>
              <button 
                @click="runDns"
                :disabled="loading || !dnsHostname"
                class="px-4 py-2 bg-blue-500 text-white rounded-lg hover:bg-blue-600 disabled:bg-gray-500 disabled:cursor-not-allowed flex items-center gap-2">
                <Loader2 v-if="loading" :size="16" class="animate-spin" />
                <Search v-else :size="16" />
                Lookup
              </button>
            </div>
          </div>

          <!-- Results -->
          <div v-if="result" class="bg-page rounded-lg border border-divider overflow-hidden">
            <div class="bg-header px-4 py-2 border-b border-divider flex items-center justify-between">
              <div class="flex items-center gap-3">
                <span class="text-sm font-medium text-primary">Result</span>
                <span v-if="result.via" class="text-xs text-secondary px-2 py-0.5 bg-button rounded">
                  via {{ result.via }}
                </span>
                <span v-if="result.tool" class="text-xs text-secondary px-2 py-0.5 bg-button rounded">
                  {{ result.tool }}
                </span>
              </div>
              <span :class="result.success ? 'text-green-400' : 'text-red-400'" class="text-xs">
                {{ result.success ? 'Success' : 'Failed' }}
              </span>
            </div>
            <pre class="p-4 text-xs font-mono text-primary overflow-x-auto whitespace-pre-wrap">{{ result.output }}</pre>
            <div v-if="result.error" class="px-4 py-2 bg-red-500/10 text-red-400 text-xs">
              {{ result.error }}
            </div>
          </div>
        </div>

        <!-- MTU Tester Tab -->
        <div v-if="activeTab === 'mtu'" class="space-y-6">
          <div class="bg-blue-500/10 border border-blue-500/30 rounded-lg p-4">
            <div class="flex items-start gap-3">
              <Info :size="20" class="text-blue-400 shrink-0 mt-0.5" />
              <div class="text-sm text-primary">
                <p class="font-medium mb-1">MTU (Maximum Transmission Unit)</p>
                <p class="text-secondary">Tests the largest packet size that can reach the target without fragmentation. Useful for optimizing WireGuard tunnel performance.</p>
              </div>
            </div>
          </div>

          <!-- Route Selection for MTU -->
          <div class="flex items-center gap-3 p-3 bg-page rounded-lg border border-divider">
            <span class="text-sm text-secondary">Test via:</span>
            <label class="flex items-center gap-2 cursor-pointer">
              <input type="radio" v-model="selectedInterface" value="" class="w-4 h-4" />
              <span class="text-sm text-primary">Default (LAN)</span>
            </label>
            <label class="flex items-center gap-2 cursor-pointer">
              <input type="radio" v-model="selectedInterface" value="wg" class="w-4 h-4" />
              <span class="text-sm text-primary">WireGuard Tunnel</span>
            </label>
          </div>

          <div class="flex gap-3 items-end">
            <div class="flex-1">
              <label class="block text-sm text-secondary mb-1">Target</label>
              <input 
                v-model="mtuTarget"
                type="text"
                placeholder="Enter peer IP or hostname"
                class="w-full px-4 py-2 bg-input border border-divider rounded-lg text-primary placeholder-secondary focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              />
            </div>
            <div>
              <label class="block text-sm text-secondary mb-1">Start MTU</label>
              <input 
                v-model.number="mtuStart"
                type="number"
                min="500"
                max="1500"
                class="w-24 px-4 py-2 bg-input border border-divider rounded-lg text-primary"
              />
            </div>
            <div>
              <label class="block text-sm text-secondary mb-1">End MTU</label>
              <input 
                v-model.number="mtuEnd"
                type="number"
                min="500"
                max="1500"
                class="w-24 px-4 py-2 bg-input border border-divider rounded-lg text-primary"
              />
            </div>
            <button 
              @click="runMtuTest"
              :disabled="loading || !mtuTarget"
              class="px-4 py-2 bg-green-500 text-white rounded-lg hover:bg-green-600 disabled:bg-gray-500 disabled:cursor-not-allowed flex items-center gap-2">
              <Loader2 v-if="loading" :size="16" class="animate-spin" />
              <Gauge v-else :size="16" />
              Test MTU
            </button>
          </div>

          <!-- MTU Results -->
          <div v-if="mtuResult" class="space-y-4">
            <div :class="[
              'rounded-lg p-4 border',
              mtuResult.optimal_mtu > 0 ? 'bg-green-500/10 border-green-500/30' : 'bg-red-500/10 border-red-500/30'
            ]">
              <div class="flex items-center gap-3">
                <CheckCircle v-if="mtuResult.optimal_mtu > 0" :size="24" class="text-green-400" />
                <XCircle v-else :size="24" class="text-red-400" />
                <div class="flex-1">
                  <div class="flex items-center gap-2">
                    <p class="font-medium text-primary">{{ mtuResult.recommendation }}</p>
                    <span v-if="mtuResult.via" class="text-xs text-secondary px-2 py-0.5 bg-button rounded">
                      via {{ mtuResult.via }}
                    </span>
                  </div>
                  <p v-if="mtuResult.optimal_mtu > 0" class="text-sm text-secondary mt-1">
                    For WireGuard, subtract 80 bytes for overhead: <span class="font-mono text-blue-400">{{ mtuResult.optimal_mtu - 80 }}</span>
                  </p>
                </div>
              </div>
            </div>

            <div class="bg-page rounded-lg border border-divider overflow-hidden">
              <div class="bg-header px-4 py-2 border-b border-divider">
                <span class="text-sm font-medium text-primary">Test Results</span>
              </div>
              <div class="p-4 grid grid-cols-4 gap-2 max-h-48 overflow-y-auto">
                <div 
                  v-for="r in mtuResult.results" 
                  :key="r.mtu"
                  :class="[
                    'px-3 py-2 rounded text-xs font-mono text-center',
                    r.success ? 'bg-green-500/20 text-green-400' : 'bg-red-500/20 text-red-400'
                  ]">
                  {{ r.mtu }}
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- Troubleshooting Wizard Tab -->
        <div v-if="activeTab === 'wizard'" class="space-y-6">
          <!-- Step indicator -->
          <div class="flex items-center justify-center gap-2">
            <div 
              v-for="(step, index) in wizardSteps" 
              :key="index"
              :class="[
                'w-8 h-8 rounded-full flex items-center justify-center text-sm font-medium transition-colors',
                wizardStep === index 
                  ? 'bg-blue-500 text-white' 
                  : wizardStep > index 
                    ? 'bg-green-500 text-white'
                    : 'bg-button text-secondary'
              ]">
              <Check v-if="wizardStep > index" :size="16" />
              <span v-else>{{ index + 1 }}</span>
            </div>
          </div>

          <!-- Step 1: Select Issue -->
          <div v-if="wizardStep === 0" class="space-y-4">
            <h3 class="text-lg font-medium text-primary text-center">What issue are you experiencing?</h3>
            <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
              <button 
                v-for="issue in issues"
                :key="issue.id"
                @click="selectIssue(issue)"
                class="p-4 bg-page border border-divider rounded-lg hover:border-blue-500 transition-colors text-left">
                <div class="flex items-start gap-3">
                  <component :is="issue.icon" :size="24" class="text-icon shrink-0" />
                  <div>
                    <p class="font-medium text-primary">{{ issue.title }}</p>
                    <p class="text-sm text-secondary mt-1">{{ issue.description }}</p>
                  </div>
                </div>
              </button>
            </div>
          </div>

          <!-- Step 2: Running Checks -->
          <div v-if="wizardStep === 1" class="space-y-4">
            <h3 class="text-lg font-medium text-primary text-center">Running diagnostics...</h3>
            <div class="flex justify-center py-8">
              <Loader2 :size="48" class="animate-spin text-blue-500" />
            </div>
            <p class="text-center text-secondary">Checking system configuration and connectivity</p>
          </div>

          <!-- Step 3: Results & Suggestions -->
          <div v-if="wizardStep === 2" class="space-y-4">
            <h3 class="text-lg font-medium text-primary text-center">Diagnostic Results</h3>
            
            <!-- Overall Status -->
            <div :class="[
              'rounded-lg p-4 border',
              wizardResults?.overall_status === 'ok' ? 'bg-green-500/10 border-green-500/30' :
              wizardResults?.overall_status === 'warning' ? 'bg-yellow-500/10 border-yellow-500/30' :
              'bg-red-500/10 border-red-500/30'
            ]">
              <div class="flex items-center gap-3">
                <CheckCircle v-if="wizardResults?.overall_status === 'ok'" :size="24" class="text-green-400" />
                <AlertTriangle v-else-if="wizardResults?.overall_status === 'warning'" :size="24" class="text-yellow-400" />
                <XCircle v-else :size="24" class="text-red-400" />
                <p class="font-medium text-primary">
                  {{ wizardResults?.overall_status === 'ok' ? 'All checks passed!' :
                     wizardResults?.overall_status === 'warning' ? 'Some warnings detected' :
                     'Issues found that need attention' }}
                </p>
              </div>
            </div>

            <!-- Individual Checks -->
            <div class="space-y-2">
              <div 
                v-for="check in wizardResults?.checks" 
                :key="check.name"
                class="flex items-center gap-3 p-3 bg-page rounded-lg border border-divider">
                <CheckCircle v-if="check.status === 'ok'" :size="20" class="text-green-400 shrink-0" />
                <AlertTriangle v-else-if="check.status === 'warning'" :size="20" class="text-yellow-400 shrink-0" />
                <Info v-else-if="check.status === 'info'" :size="20" class="text-blue-400 shrink-0" />
                <XCircle v-else :size="20" class="text-red-400 shrink-0" />
                <div class="flex-1 min-w-0">
                  <p class="font-medium text-primary text-sm">{{ check.name }}</p>
                  <p class="text-xs text-secondary truncate">{{ check.message }}</p>
                </div>
              </div>
            </div>

            <!-- Suggestions -->
            <div v-if="wizardResults?.suggestions?.length > 0" class="bg-blue-500/10 border border-blue-500/30 rounded-lg p-4">
              <p class="font-medium text-primary mb-2 flex items-center gap-2">
                <Lightbulb :size="18" class="text-yellow-400" />
                Suggestions
              </p>
              <ul class="space-y-2">
                <li v-for="(suggestion, index) in wizardResults.suggestions" :key="index" class="text-sm text-secondary flex items-start gap-2">
                  <span class="text-blue-400">•</span>
                  {{ suggestion }}
                </li>
              </ul>
            </div>

            <!-- Actions -->
            <div class="flex justify-center gap-3 pt-4">
              <button 
                @click="wizardStep = 0; selectedIssue = null; wizardResults = null"
                class="px-4 py-2 bg-button text-button rounded-lg hover:bg-button-hover">
                Start Over
              </button>
              <button 
                @click="runWizardChecks"
                class="px-4 py-2 bg-blue-500 text-white rounded-lg hover:bg-blue-600 flex items-center gap-2">
                <RefreshCw :size="16" />
                Re-run Checks
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- Footer -->
      <div class="px-6 py-3 border-t border-divider flex items-center justify-end">
        <button 
          @click="$emit('close')"
          class="px-4 py-2 bg-button text-button rounded-lg hover:bg-button-hover transition-colors">
          Close
        </button>
      </div>
    </div>
  </div>
</template>

<script>
import { 
  Stethoscope, X, Play, Search, Loader2, Info, Gauge, 
  CheckCircle, XCircle, Check, AlertTriangle, Lightbulb, RefreshCw,
  Wifi, WifiOff, Globe, Server, Network
} from 'lucide-vue-next';

export default {
  name: 'DiagnosticsDialog',
  components: {
    Stethoscope, X, Play, Search, Loader2, Info, Gauge,
    CheckCircle, XCircle, Check, AlertTriangle, Lightbulb, RefreshCw,
    Wifi, WifiOff, Globe, Server, Network
  },
  props: {
    api: {
      type: Object,
      required: true
    },
    peers: {
      type: Object,
      default: () => ({})
    }
  },
  emits: ['close'],
  data() {
    return {
      activeTab: 'network',
      tabs: [
        { id: 'network', name: 'Network Tools', icon: 'Globe' },
        { id: 'mtu', name: 'MTU Tester', icon: 'Gauge' },
        { id: 'wizard', name: 'Troubleshooter', icon: 'Stethoscope' }
      ],
      networkTools: [
        { id: 'ping', name: 'Ping' },
        { id: 'traceroute', name: 'Traceroute' },
        { id: 'dns', name: 'DNS Lookup' }
      ],
      selectedTool: 'ping',
      selectedInterface: '', // '' = default route, 'wg' = WireGuard tunnel
      loading: false,
      result: null,
      
      // Ping
      pingTarget: '',
      pingCount: 4,
      
      // Traceroute
      tracerouteTarget: '',
      tracerouteHops: 20,
      
      // DNS
      dnsHostname: '',
      dnsServer: '',
      dnsType: '',
      
      // MTU
      mtuTarget: '',
      mtuStart: 1500,
      mtuEnd: 1280,
      mtuResult: null,
      
      // Wizard
      wizardStep: 0,
      wizardSteps: ['Select Issue', 'Running Checks', 'Results'],
      selectedIssue: null,
      wizardResults: null,
      issues: [
        { 
          id: 'peer_offline', 
          title: 'Peer appears offline',
          description: 'A peer shows as disconnected or has no recent handshake',
          icon: 'WifiOff'
        },
        { 
          id: 'slow_connection', 
          title: 'Slow connection',
          description: 'Traffic is slow or experiencing high latency',
          icon: 'Wifi'
        },
        { 
          id: 'no_internet', 
          title: 'No internet through tunnel',
          description: 'Can\'t reach internet when using gateway peer',
          icon: 'Globe'
        },
        { 
          id: 'general', 
          title: 'General check',
          description: 'Run all diagnostic checks',
          icon: 'Server'
        }
      ]
    };
  },
  methods: {
    async runPing() {
      if (!this.pingTarget || this.loading) return;
      this.loading = true;
      this.result = null;
      try {
        this.result = await this.api.diagnostics_ping(this.pingTarget, this.pingCount, this.selectedInterface || null);
      } catch (err) {
        this.result = { success: false, output: '', error: err.message };
      } finally {
        this.loading = false;
      }
    },
    async runTraceroute() {
      if (!this.tracerouteTarget || this.loading) return;
      this.loading = true;
      this.result = null;
      try {
        this.result = await this.api.diagnostics_traceroute(this.tracerouteTarget, this.tracerouteHops, this.selectedInterface || null);
      } catch (err) {
        this.result = { success: false, output: '', error: err.message };
      } finally {
        this.loading = false;
      }
    },
    async runDns() {
      if (!this.dnsHostname || this.loading) return;
      this.loading = true;
      this.result = null;
      try {
        this.result = await this.api.diagnostics_dns(
          this.dnsHostname, 
          this.dnsServer || null, 
          this.dnsType || null
        );
      } catch (err) {
        this.result = { success: false, output: '', error: err.message };
      } finally {
        this.loading = false;
      }
    },
    async runMtuTest() {
      if (!this.mtuTarget || this.loading) return;
      this.loading = true;
      this.mtuResult = null;
      try {
        this.mtuResult = await this.api.diagnostics_mtu(this.mtuTarget, this.mtuStart, this.mtuEnd, this.selectedInterface || null);
      } catch (err) {
        this.mtuResult = { optimal_mtu: 0, results: [], recommendation: err.message };
      } finally {
        this.loading = false;
      }
    },
    selectIssue(issue) {
      this.selectedIssue = issue;
      this.runWizardChecks();
    },
    async runWizardChecks() {
      this.wizardStep = 1;
      this.wizardResults = null;
      
      try {
        // Run peer diagnostics
        this.wizardResults = await this.api.diagnostics_peer_check(this.selectedIssue?.id || 'general');
      } catch (err) {
        this.wizardResults = {
          overall_status: 'error',
          checks: [{ name: 'API Error', status: 'error', message: err.message }],
          suggestions: ['Check if the wg-quickrs service is running']
        };
      }
      
      this.wizardStep = 2;
    },
    handleKeydown(e) {
      if (e.key === 'Escape') {
        this.$emit('close');
      }
    }
  },
  mounted() {
    document.addEventListener('keydown', this.handleKeydown);
  },
  beforeUnmount() {
    document.removeEventListener('keydown', this.handleKeydown);
  }
};
</script>
