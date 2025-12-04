<template>

  <div class="text-sm text-secondary whitespace-pre grid grid-cols-2 gap-1">
    <div v-if="is_full(pruned_errors)" class="col-span-2 bg-badge-error-bg rounded-md overflow-scroll">
      <div class="flex items-center justify-center bg-badge-error-bg rounded-md p-1">
        <strong class="text-badge-error-text">Errors</strong>
      </div>
      <div class="text-sm text-primary p-1">
        {{ JSON.stringify(pruned_errors, false, 2) }}
      </div>
    </div>
    <div v-if="is_full(pruned_changed_fields)" class="col-span-2 bg-badge-info-bg rounded-md overflow-scroll">
      <div class="flex items-center justify-center bg-badge-info-bg rounded-md p-1">
        <strong class="text-badge-info-text">Changed Fields</strong>
      </div>
      <div class="text-sm text-primary p-1">
        {{ JSON.stringify(pruned_changed_fields, false, 2) }}
      </div>
    </div>
    <div v-if="is_full(changeSum.added_peers)" class="col-span-2 bg-badge-success-bg rounded-md overflow-scroll">
      <div class="flex items-center justify-center bg-badge-success-bg rounded-md p-1">
        <strong class="text-badge-success-text">Added Peer</strong>
      </div>
      <div class="text-sm text-primary p-1">
        {{ JSON.stringify(padded_added_peers, false, 2) }}
      </div>
    </div>
    <div v-if="is_full(changeSum.added_connections)" class="col-span-2 bg-badge-success-bg rounded-md overflow-scroll">
      <div class="flex items-center justify-center bg-badge-success-bg rounded-md p-1">
        <strong class="text-badge-success-text">Added Connections</strong>
      </div>
      <div class="text-sm text-primary p-1">
        {{ JSON.stringify(changeSum.added_connections, false, 2) }}
      </div>
    </div>
    <div v-if="is_full(changeSum.removed_connections)" class="col-span-2 bg-badge-error-bg rounded-md overflow-scroll">
      <div class="flex items-center justify-center bg-badge-error-bg rounded-md p-1">
        <strong class="text-badge-error-text">Removed Connections</strong>
      </div>
      <div class="text-sm text-primary p-1">
        {{ JSON.stringify(changeSum.removed_connections, false, 2) }}
      </div>
    </div>
    <div v-if="Object.keys(network.peers).includes(peerId)" class="bg-card rounded-md overflow-scroll border border-divider">
      <div class="flex items-center justify-center bg-button rounded-md p-1">
        <strong class="text-secondary">Old Configuration</strong>
      </div>
      <span class="text-sm text-primary p-1">
        {{ JSON.stringify({peers: pruned_network.peers, connections: pruned_network.connections}, false, 2) }}
      </span>
    </div>
    <div v-if="Object.keys(network.peers).includes(peerId)" class="bg-badge-success-bg rounded-md overflow-scroll">
      <div class="flex items-center justify-center bg-badge-success-bg rounded-md p-1">
        <strong class="text-badge-success-text">New Configuration</strong>
      </div>
      <span class="text-sm text-primary p-1">
        {{ JSON.stringify({peers: new_network.peers, connections: new_network.connections}, false, 2) }}
      </span>
    </div>
  </div>

</template>

<script>


export default {
  name: "change-sum",
  props: {
    changeSum: {
      type: Object,
      default: {
        errors: {
          peers: {},
          connections: {},
        },
        changed_fields: {
          peers: {},
          connections: {},
        },
        added_connections: {},
        removed_connections: {}
      },
    },
    peerId: {
      type: String,
      default: "",
    },
    network: {
      type: Object,
      default: {},
    },
  },
  methods: {
    is_full(field) {
      if (field === undefined) {
        return false;
      }
      return Object.keys(field).length !== 0;

    }
  },
  computed: {
    pruned_errors() {
      const pruned_errors = {};
      if (this.changeSum.errors.peers[this.peerId]) {
        if (Object.keys(this.changeSum.errors.peers[this.peerId]).length) {
          pruned_errors.peers = this.changeSum.errors.peers;
        }
      }
      if (Object.keys(this.changeSum.errors.connections).length) {
        pruned_errors.connections = this.changeSum.errors.connections;
      }
      return pruned_errors;
    },
    pruned_changed_fields() {
      if (!Object.keys(this.changeSum).includes("changed_fields")) {
        return {};
      }

      const pruned_changed_fields = {};
      if (Object.keys(this.changeSum.changed_fields.peers[this.peerId]).length) {
        pruned_changed_fields.peers = this.changeSum.changed_fields.peers;
      }
      if (Object.keys(this.changeSum.changed_fields.connections).length) {
        pruned_changed_fields.connections = this.changeSum.changed_fields.connections;
      }
      return pruned_changed_fields;
    },
    padded_added_peers() {
      return {peers: this.changeSum.added_peers};
    },
    pruned_network() {
      let pruned_network = {peers: {}, connections: {}};
      pruned_network.peers[this.peerId] = this.network.peers[this.peerId];

      for (let connection_id in this.network.connections) {
        if (connection_id.includes(this.peerId)) {
          pruned_network.connections[connection_id] = this.network.connections[connection_id];
        }
      }
      return pruned_network;
    },
    new_network() {
      let new_network = JSON.parse(JSON.stringify(this.pruned_network));

      if (this.changeSum.changed_fields) {
        // peer changes
        for (const peer_id in this.changeSum.changed_fields.peers) {
          for (const peer_field in this.changeSum.changed_fields.peers[peer_id]) {
            if (peer_field === "scripts") {
              for (const script_field in this.changeSum.changed_fields.peers[peer_id].scripts) {
                new_network.peers[peer_id].scripts[script_field] = this.changeSum.changed_fields.peers[peer_id].scripts[script_field];
              }
              continue;
            }
            new_network.peers[peer_id][peer_field] = this.changeSum.changed_fields.peers[peer_id][peer_field];
          }
        }

        // connection changes
        for (const connection_id in this.changeSum.changed_fields.connections) {
          for (const connection_field in this.changeSum.changed_fields.connections[connection_id]) {
            new_network.connections[connection_id][connection_field] = this.changeSum.changed_fields.connections[connection_id][connection_field];
          }
        }
      }

      if (this.changeSum.added_connections) {
        for (const added_connection_id in this.changeSum.added_connections) {
          new_network.connections[added_connection_id] = this.changeSum.added_connections[added_connection_id];
        }
      }

      if (this.changeSum.removed_connections) {
        for (const removed_connection_id in this.changeSum.removed_connections) {
          delete new_network.connections[removed_connection_id];
        }
      }

      return new_network;
    },
  }
}
</script>

<style scoped>

</style>