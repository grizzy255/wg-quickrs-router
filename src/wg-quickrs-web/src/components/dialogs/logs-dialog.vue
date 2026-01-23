<template>
  <div class="fixed inset-0 bg-backdrop z-50 flex items-center justify-center p-4" @click.self="$emit('close')">
    <div class="bg-card rounded-lg shadow-xl w-full max-w-4xl max-h-[85vh] flex flex-col border border-divider">
      <!-- Header -->
      <div class="flex items-center justify-between px-6 py-4 border-b border-divider">
        <div class="flex items-center gap-3">
          <ScrollText :size="24" class="text-icon" />
          <h2 class="text-xl font-semibold text-primary">System Logs</h2>
        </div>
        <div class="flex items-center gap-3">
          <!-- Line count selector -->
          <select 
            v-model="lineCount" 
            @change="fetchLogs"
            class="px-3 py-1.5 text-sm bg-input border border-input rounded-md text-primary focus:ring-2 focus:ring-blue-500">
            <option :value="50">50 lines</option>
            <option :value="100">100 lines</option>
            <option :value="250">250 lines</option>
            <option :value="500">500 lines</option>
            <option :value="1000">1000 lines</option>
          </select>
          <!-- Refresh button -->
          <button 
            @click="fetchLogs" 
            :disabled="loading"
            class="px-3 py-1.5 text-sm bg-button hover:bg-button-hover text-button rounded-md flex items-center gap-2 transition-colors">
            <RefreshCw :size="16" :class="{ 'animate-spin': loading }" />
            Refresh
          </button>
          <!-- Close button -->
          <button @click="$emit('close')" class="text-secondary hover:text-primary transition-colors">
            <X :size="24" />
          </button>
        </div>
      </div>

      <!-- Search bar -->
      <div class="px-6 py-3 border-b border-divider">
        <div class="relative">
          <Search :size="18" class="absolute left-3 top-1/2 -translate-y-1/2 text-muted" />
          <input
            v-model="searchQuery"
            type="text"
            placeholder="Filter logs... (e.g., error, warn, peer name)"
            class="w-full pl-10 pr-4 py-2 bg-input border border-input rounded-md text-primary placeholder-muted focus:ring-2 focus:ring-blue-500 focus:border-transparent"
          />
        </div>
      </div>

      <!-- Log content -->
      <div class="flex-1 overflow-auto p-4 bg-page">
        <div v-if="loading" class="flex items-center justify-center h-full">
          <div class="text-secondary">Loading logs...</div>
        </div>
        <div v-else-if="error" class="flex items-center justify-center h-full">
          <div class="text-red-500">{{ error }}</div>
        </div>
        <div v-else class="font-mono text-xs leading-relaxed">
          <div 
            v-for="(line, index) in filteredLogs" 
            :key="index"
            :class="getLogLineClass(line)"
            class="py-0.5 px-2 rounded hover:bg-button/50 whitespace-pre-wrap break-all">
            {{ line }}
          </div>
          <div v-if="filteredLogs.length === 0 && logs.length > 0" class="text-secondary text-center py-8">
            No logs match your filter
          </div>
          <div v-if="logs.length === 0" class="text-secondary text-center py-8">
            No logs available
          </div>
        </div>
      </div>

      <!-- Footer -->
      <div class="px-6 py-3 border-t border-divider flex items-center justify-between text-sm text-secondary">
        <div>
          Showing {{ filteredLogs.length }} of {{ logs.length }} lines
        </div>
        <div class="flex items-center gap-4">
          <label class="flex items-center gap-2 cursor-pointer">
            <input type="checkbox" v-model="autoScroll" class="w-4 h-4" />
            <span>Auto-scroll to bottom</span>
          </label>
          <span v-if="lastUpdate">Updated: {{ lastUpdate }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

<script>
import { ScrollText, X, RefreshCw, Search } from 'lucide-vue-next';

export default {
  name: 'LogsDialog',
  components: {
    ScrollText,
    X,
    RefreshCw,
    Search
  },
  props: {
    api: {
      type: Object,
      required: true
    }
  },
  emits: ['close'],
  data() {
    return {
      logs: [],
      loading: false,
      error: null,
      searchQuery: '',
      lineCount: 100,
      autoScroll: true,
      lastUpdate: null,
      refreshInterval: null
    };
  },
  computed: {
    filteredLogs() {
      if (!this.searchQuery.trim()) {
        return this.logs;
      }
      const query = this.searchQuery.toLowerCase();
      return this.logs.filter(line => line.toLowerCase().includes(query));
    }
  },
  methods: {
    async fetchLogs() {
      this.loading = true;
      this.error = null;
      
      try {
        const result = await this.api.get_system_logs(this.lineCount);
        if (result.logs) {
          this.logs = result.logs.split('\n').filter(line => line.trim());
          this.lastUpdate = new Date().toLocaleTimeString();
          
          if (this.autoScroll) {
            this.$nextTick(() => {
              const container = this.$el.querySelector('.overflow-auto');
              if (container) {
                container.scrollTop = container.scrollHeight;
              }
            });
          }
        }
      } catch (err) {
        this.error = err.message || 'Failed to fetch logs';
      } finally {
        this.loading = false;
      }
    },
    getLogLineClass(line) {
      const lowerLine = line.toLowerCase();
      if (lowerLine.includes('error') || lowerLine.includes('fatal')) {
        return 'text-red-400 bg-red-500/10';
      }
      if (lowerLine.includes('warn')) {
        return 'text-yellow-400 bg-yellow-500/10';
      }
      if (lowerLine.includes('info')) {
        return 'text-primary';
      }
      if (lowerLine.includes('debug')) {
        return 'text-secondary';
      }
      return 'text-primary';
    },
    handleKeydown(e) {
      if (e.key === 'Escape') {
        this.$emit('close');
      }
    }
  },
  mounted() {
    this.fetchLogs();
    // Auto-refresh every 5 seconds
    this.refreshInterval = setInterval(() => {
      if (!this.loading) {
        this.fetchLogs();
      }
    }, 5000);
    
    document.addEventListener('keydown', this.handleKeydown);
  },
  beforeUnmount() {
    if (this.refreshInterval) {
      clearInterval(this.refreshInterval);
    }
    document.removeEventListener('keydown', this.handleKeydown);
  }
};
</script>

