<template>
  <div class="fixed inset-0 flex items-center justify-center z-50">
    <!-- Backdrop -->
    <div class="fixed inset-0 bg-backdrop"></div>

    <!-- Modal Card -->
    <div aria-modal="true"
         :class="modalClasses"
         class="relative bg-card rounded-lg shadow-sm border border-divider overflow-hidden text-left transform transition-all w-full mx-4 max-h-[90vh] flex flex-col"
         role="dialog">
      <div class="px-6 pt-6 pb-4 flex-1 overflow-y-auto">
        <div class="pr-2">
          <div v-if="icon === 'danger'"
               class="mx-auto flex-shrink-0 flex items-center justify-center h-12 w-12 rounded-full bg-badge-error-bg sm:mx-0 sm:h-10 sm:w-10 mb-4">
            <svg aria-hidden="true" class="h-6 w-6 text-badge-error-text fill-none" stroke="currentColor"
                 viewBox="0 0 24 24">
              <path
                  d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0
                 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464
                 0L3.34 16c-.77 1.333.192 3 1.732 3z"
                  stroke-linecap="round" stroke-linejoin="round"
                  stroke-width="2"/>
            </svg>
          </div>
          <div v-else-if="icon === 'warning'"
               class="mx-auto flex-shrink-0 flex items-center justify-center h-12 w-12 rounded-full bg-badge-warning-bg sm:mx-0 sm:h-10 sm:w-10 mb-4">
            <svg aria-hidden="true" class="h-6 w-6 text-badge-warning-text" fill="currentColor" viewBox="0 0 20 20">
              <path fill-rule="evenodd" d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z" clip-rule="evenodd"/>
            </svg>
          </div>

          <div class="text-center sm:text-left w-full">
            <slot></slot>
          </div>
        </div>
      </div>

      <div class="bg-button px-6 py-4 border-t border-divider flex flex-row-reverse gap-3">
        <button v-if="rightButtonText" :class="[rightButtonClasses, rightButtonColor ? '' : 'text-primary']" :disabled="rightButtonDisabled"
                class="inline-flex justify-center rounded-md border shadow-sm px-4 py-2 text-sm font-medium bg-button hover:bg-button-hover border-divider disabled:cursor-not-allowed disabled:text-muted disabled:opacity-50"
                type="button" @click="rightButtonClick">
          {{ rightButtonText }}
        </button>
        <button v-if="leftButtonText" :disabled="leftButtonDisabled"
                class="inline-flex justify-center rounded-md border border-divider shadow-sm px-4 py-2 bg-card text-sm font-medium text-primary hover:bg-button disabled:cursor-not-allowed disabled:text-muted disabled:opacity-50"
                type="button" @click="leftButtonClick">
          {{ leftButtonText }}
        </button>
      </div>
    </div>
  </div>
</template>

<script>
export default {
  name: "custom-dialog",
  props: {
    rightButtonText: {
      type: String,
      default: 'Approve',
    },
    rightButtonColor: {
      type: String,
      default: '',
    },
    rightButtonDisabled: {
      type: Boolean,
      default: false,
    },
    rightButtonClick: {
      type: Function,
      default: () => {
      },
    },
    leftButtonText: {
      type: String,
      default: 'Cancel',
    },
    leftButtonDisabled: {
      type: Boolean,
      default: false,
    },
    leftButtonClick: {
      type: Function,
      default: () => {
      },
    },
    icon: {
      type: String,
      default: '',
    },
    modalClasses: {
      type: String,
      default: 'max-w-4xl',
    }
  },
  computed: {
    rightButtonClasses() {
      if (this.rightButtonColor === 'green') {
        return ['enabled:text-green-50', 'enabled:bg-green-700', 'enabled:hover:text-green-50', 'enabled:border-green-900', 'enabled:hover:bg-green-600', 'enabled:hover:border-green-600'];
      } else if (this.rightButtonColor === 'red') {
        return ['enabled:text-red-800', 'enabled:bg-red-100', 'enabled:hover:text-red-50', 'enabled:hover:bg-red-600', 'enabled:border-red-300', 'enabled:hover:border-red-600'];
      } else if (this.rightButtonColor === 'blue') {
        return ['enabled:text-blue-50', 'enabled:bg-blue-700', 'enabled:hover:text-blue-50', 'enabled:border-blue-900', 'enabled:hover:bg-blue-600', 'enabled:hover:border-blue-600'];
      }

      return [];
    }
  }
}
</script>

<style scoped>
</style>