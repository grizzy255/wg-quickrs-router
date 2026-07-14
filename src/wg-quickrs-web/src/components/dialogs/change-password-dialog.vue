<template>
  <div class="fixed inset-0 bg-backdrop z-50 flex items-center justify-center p-4" @click.self="$emit('close')">
    <div class="bg-card rounded-lg shadow-xl w-full max-w-md flex flex-col border border-divider">
      <!-- Header -->
      <div class="flex items-center justify-between px-6 py-4 border-b border-divider">
        <div class="flex items-center gap-3">
          <KeyRound :size="24" class="text-icon" />
          <h2 class="text-xl font-semibold text-primary">{{ hasPassword ? 'Change Password' : 'Set Password' }}</h2>
        </div>
        <button @click="$emit('close')" class="text-secondary hover:text-primary transition-colors">
          <X :size="24" />
        </button>
      </div>

      <!-- Loading state -->
      <div v-if="loadingStatus" class="p-6 flex items-center justify-center">
        <Loader2 :size="24" class="animate-spin text-secondary" />
        <span class="ml-2 text-secondary">Loading...</span>
      </div>

      <!-- Content -->
      <form v-else @submit.prevent="handleSubmit" class="p-6 space-y-4">
        <!-- Info message when no password is set -->
        <div v-if="!hasPassword" class="bg-blue-500/10 border border-blue-500/30 rounded-lg p-3">
          <p class="text-blue-400 text-sm">No password is currently configured. Set a password to secure access.</p>
        </div>

        <!-- Current Password (only if password exists) -->
        <div v-if="hasPassword">
          <label class="block text-sm font-medium text-secondary mb-1">Current Password</label>
          <div class="relative">
            <input 
              :type="showCurrentPassword ? 'text' : 'password'"
              v-model="currentPassword"
              placeholder="Enter current password"
              class="w-full px-4 py-2 pr-10 bg-input border border-divider rounded-lg text-primary placeholder-secondary focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              required
            />
            <button 
              type="button"
              @click="showCurrentPassword = !showCurrentPassword"
              class="absolute right-3 top-1/2 -translate-y-1/2 text-secondary hover:text-primary">
              <EyeOff v-if="showCurrentPassword" :size="18" />
              <Eye v-else :size="18" />
            </button>
          </div>
        </div>

        <!-- New Password -->
        <div>
          <label class="block text-sm font-medium text-secondary mb-1">New Password</label>
          <div class="relative">
            <input 
              :type="showNewPassword ? 'text' : 'password'"
              v-model="newPassword"
              placeholder="Enter new password"
              class="w-full px-4 py-2 pr-10 bg-input border border-divider rounded-lg text-primary placeholder-secondary focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              required
              minlength="4"
            />
            <button 
              type="button"
              @click="showNewPassword = !showNewPassword"
              class="absolute right-3 top-1/2 -translate-y-1/2 text-secondary hover:text-primary">
              <EyeOff v-if="showNewPassword" :size="18" />
              <Eye v-else :size="18" />
            </button>
          </div>
        </div>

        <!-- Confirm Password -->
        <div>
          <label class="block text-sm font-medium text-secondary mb-1">Confirm New Password</label>
          <div class="relative">
            <input 
              :type="showConfirmPassword ? 'text' : 'password'"
              v-model="confirmPassword"
              placeholder="Confirm new password"
              class="w-full px-4 py-2 pr-10 bg-input border border-divider rounded-lg text-primary placeholder-secondary focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              required
            />
            <button 
              type="button"
              @click="showConfirmPassword = !showConfirmPassword"
              class="absolute right-3 top-1/2 -translate-y-1/2 text-secondary hover:text-primary">
              <EyeOff v-if="showConfirmPassword" :size="18" />
              <Eye v-else :size="18" />
            </button>
          </div>
          <p v-if="confirmPassword && newPassword !== confirmPassword" class="text-red-400 text-xs mt-1">
            Passwords do not match
          </p>
        </div>

        <!-- Password requirements -->
        <div class="text-xs text-secondary">
          Password must be at least 4 characters long.
        </div>

        <!-- Error message -->
        <div v-if="error" class="bg-red-500/10 border border-red-500/30 rounded-lg p-3">
          <p class="text-red-400 text-sm">{{ error }}</p>
        </div>

        <!-- Success message -->
        <div v-if="success" class="bg-green-500/10 border border-green-500/30 rounded-lg p-3">
          <p class="text-green-400 text-sm">{{ success }}</p>
        </div>
      </form>

      <!-- Footer -->
      <div class="px-6 py-4 border-t border-divider flex items-center justify-end gap-3">
        <button 
          @click="$emit('close')"
          class="px-4 py-2 bg-button text-button rounded-lg hover:bg-button-hover transition-colors">
          Cancel
        </button>
        <button 
          @click="handleSubmit"
          :disabled="loading || !isValid"
          class="px-4 py-2 bg-blue-500 text-white rounded-lg hover:bg-blue-600 disabled:bg-gray-500 disabled:cursor-not-allowed transition-colors flex items-center gap-2">
          <Loader2 v-if="loading" :size="16" class="animate-spin" />
          <Save v-else :size="16" />
          {{ hasPassword ? 'Change Password' : 'Set Password' }}
        </button>
      </div>
    </div>
  </div>
</template>

<script>
import { KeyRound, X, Eye, EyeOff, Loader2, Save } from 'lucide-vue-next';

export default {
  name: 'ChangePasswordDialog',
  components: {
    KeyRound,
    X,
    Eye,
    EyeOff,
    Loader2,
    Save
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
      currentPassword: '',
      newPassword: '',
      confirmPassword: '',
      showCurrentPassword: false,
      showNewPassword: false,
      showConfirmPassword: false,
      loading: false,
      loadingStatus: true,
      hasPassword: false,
      error: null,
      success: null
    };
  },
  computed: {
    isValid() {
      const currentPasswordValid = this.hasPassword ? this.currentPassword.length > 0 : true;
      return currentPasswordValid &&
             this.newPassword.length >= 4 &&
             this.newPassword === this.confirmPassword;
    }
  },
  methods: {
    async fetchPasswordStatus() {
      try {
        const response = await this.api.get_password_status();
        this.hasPassword = response.has_password;
      } catch (err) {
        // If we can't fetch status, assume password exists (safer)
        this.hasPassword = true;
      } finally {
        this.loadingStatus = false;
      }
    },
    async handleSubmit() {
      if (!this.isValid || this.loading) return;
      
      this.loading = true;
      this.error = null;
      this.success = null;
      
      try {
        const currentPwd = this.hasPassword ? this.currentPassword : null;
        await this.api.change_password(currentPwd, this.newPassword);
        this.success = this.hasPassword 
          ? 'Password changed successfully! You will need to log in again with your new password.'
          : 'Password set successfully! You will need to log in with your new password.';
        
        // Clear form
        this.currentPassword = '';
        this.newPassword = '';
        this.confirmPassword = '';
        
        // Close dialog after a delay and logout
        setTimeout(() => {
          this.$emit('close');
          // Trigger logout to force re-authentication with new password
          this.api.token = '';
          localStorage.removeItem('token');
          localStorage.removeItem('remember');
          window.location.reload();
        }, 2000);
      } catch (err) {
        this.error = err.message || 'Failed to change password';
      } finally {
        this.loading = false;
      }
    },
    handleKeydown(e) {
      if (e.key === 'Escape') {
        this.$emit('close');
      }
    }
  },
  async mounted() {
    document.addEventListener('keydown', this.handleKeydown);
    await this.fetchPasswordStatus();
  },
  beforeUnmount() {
    document.removeEventListener('keydown', this.handleKeydown);
  }
};
</script>
