<template>

  <div :class="[colors.div]"
      class="bg-card rounded-lg shadow-sm border border-divider p-5 mb-6 relative">
    <div class="overflow-x-auto text-lg whitespace-nowrap">
      <div class="mt-1 flex items-center">
        <field class="inline-block" field="PublicKey  :"></field>
        <refresh-button title="Generate New Private Key" @click="refreshPeerEditKeys()"></refresh-button>
        <span class="text-primary" v-if="peer_local_private_key && peer_local_private_key.trim() !== ''">{{ peer_local_public_key }}</span>
        <span class="text-muted italic" v-else>(Will be generated from private key)</span>
      </div>
      <div class="mt-1 flex items-center">
        <field class="inline-block" field="PrivateKey:"></field>
        <refresh-button title="Generate New Private Key" @click="refreshPeerEditKeys()"></refresh-button>
        <!-- Editable input for new peers, read-only display for existing peers -->
        <input
          v-if="isNewPeer"
          v-model="peer_local_private_key"
          type="text"
          placeholder="Paste your private key or leave empty to auto-generate"
          class="ml-2 rounded pl-1.5 pt-[2px] pb-[2px] focus:outline-none focus:ring-0 border-1 border-input focus:border-input-focus outline-none text-lg text-secondary grow bg-input"
          @input="onPrivateKeyChange"
        />
        <span v-else class="text-primary">{{ peer_local_private_key }}</span>
      </div>
      <div v-show="peer.created_at" class="mt-1">
        <field class="inline-block" field="CreatedAt  :"></field>
        <div class="text-primary ml-2 inline-block">
          {{ new Date(peer.created_at).toString() }}
        </div>
      </div>
      <div v-show="peer.updated_at" class="mt-1">
        <field class="inline-block" field="UpdatedAt  :"></field>
        <div class="text-primary ml-2 inline-block">
          {{ new Date(peer.updated_at).toString() }}
        </div>
      </div>
    </div>

    <!-- Undo Button -->
    <undo-button v-if="are_keys_updated"
                 :disabled="!are_keys_updated"
                 alignment-classes="right-[6px] top-[6px]"
                 image-classes="h-7"
                 @click="peer_local_private_key = peer.private_key; $emit('updated-change-sum', {changed_fields: {}, errors: {}},
    )">
    </undo-button>
  </div>

</template>

<script>
import Field from "@/src/components/ui/field.vue";
import UndoButton from "@/src/components/ui/buttons/undo.vue";
import RefreshButton from "@/src/components/ui/buttons/refresh.vue";
import WireGuardHelper from "@/src/js/wg-helper.js";
import {
  wg_generate_key_wasm,
  wg_public_key_from_private_key_wasm,
} from '@/pkg/wg_quickrs_lib.js';

export default {
  name: "peer-details-island",
  components: {RefreshButton, UndoButton, Field},
  props: {
    peer: {
      type: Object,
      default: {},
    },
    api: {
      type: Object,
      default: null,
    },
    isNewPeer: {
      type: Boolean,
      default: false,
    },
  },
  data() {
    return {
      peer_local_private_key: {},
      DIV_COLOR_LOOKUP: null,
      colors: {div: null},
    };
  },
  created() {
    // For new peers, use the existing private key from default_peer_conf (auto-generated)
    // For existing peers, use the existing private key
    this.peer_local_private_key = this.peer.private_key || '';
    this.DIV_COLOR_LOOKUP = WireGuardHelper.get_div_colors(this.isNewPeer);
  },
  emits: ['updated-change-sum'],
  methods: {
    async refreshPeerEditKeys() {
      this.peer_local_private_key = wg_generate_key_wasm();
      if (this.isNewPeer) {
        this.onPrivateKeyChange();
      }
    },
    onPrivateKeyChange() {
      // Emit the change, but only include private_key if it's provided (not empty)
      const changed_fields = {};
      if (this.peer_local_private_key && this.peer_local_private_key.trim() !== '') {
        changed_fields.private_key = this.peer_local_private_key.trim();
      }
      // If empty, don't include it - backend will auto-generate
      this.$emit("updated-change-sum", {
        changed_fields: changed_fields,
        errors: {},
      });
    }
  },
  computed: {
    are_keys_updated() {
      const keys_updated = this.peer_local_private_key !== this.peer.private_key;
      this.colors.div = keys_updated ? this.DIV_COLOR_LOOKUP.changed : this.DIV_COLOR_LOOKUP.unchanged;
      return keys_updated;
    },
    peer_local_public_key() {
      if (!this.peer_local_private_key || this.peer_local_private_key.trim() === '') {
        return '';
      }
      try {
        return wg_public_key_from_private_key_wasm(this.peer_local_private_key);
      } catch (e) {
        return '(Invalid private key)';
      }
    }
  },
}
</script>

<style scoped>

</style>