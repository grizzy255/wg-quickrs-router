'use strict';

export default class API {
    token;
    does_need_auth = false;

    async call({method, path, headers, body, signal}) {
        if (this.does_need_auth) {
            throw new Error(`A valid token required for ${method} ${path}!`);
        }

        headers = headers ? headers : {};
        if (this.token !== '') {
            headers["Authorization"] = `Bearer ${this.token}`;
        }
        
        // Add cache-busting query parameter to prevent stale responses
        const separator = path.includes('?') ? '&' : '?';
        const cacheBuster = `_t=${Date.now()}`;
        const url = `${import.meta.env.VITE_API_FETCH_URL_PREFIX}${path}${separator}${cacheBuster}`;
        
        const res = await fetch(url, {
            method,
            headers: headers,
            body: body
                ? JSON.stringify(body)
                : undefined,
            signal: signal, // Support AbortController for timeouts
        });

        // get a new token
        if (res.status === 401) {
            console.error(`Unauthorized: ${res.status}`);
            this.does_need_auth = true;
        }

        if (res.status === 204) {
            return undefined;
        }

        // Check if response is JSON before parsing
        const contentType = res.headers.get('content-type');
        if (!contentType || !contentType.includes('application/json')) {
            // If not JSON (e.g., HTML 404 page), throw a proper error
            if (!res.ok) {
                throw new Error(`Server returned ${res.status} ${res.statusText}`);
            }
            throw new Error('Server returned non-JSON response');
        }

        const json = await res.json();

        if (!res.ok) {
            throw new Error(json.error || res.statusText);
        }

        return json;
    }

    async update_api_token(password) {
        const token_res = await fetch(`${import.meta.env.VITE_API_FETCH_URL_PREFIX}/api/token`, {
            method: "post",
            body: JSON.stringify({client_id: 'web', password}),
        });
        const token = await token_res.text();
        if (token_res.status === 200) {
            this.does_need_auth = false;
            this.token = token;
        } else {
            throw new Error("Unauthorized access");
        }
    }

    async get_network_summary(url_encoded_params) {
        return this.call({
            method: 'get',
            path: `/api/network/summary${url_encoded_params}`,
            headers: {'Content-Type': 'application/x-www-form-urlencoded'}
        });
    }

    async patch_network_config(change_sum) {
        return this.call({
            method: 'patch',
            path: `/api/network/config`,
            body: change_sum
        });
    }

    async post_network_reserve_address() {
        return this.call({
            method: 'post',
            path: `/api/network/reserve/address`,
        });
    }

    async post_wireguard_status(body) {
        return this.call({
            method: 'post',
            path: '/api/wireguard/status',
            headers: {"Content-Type": "application/json"},
            body: body
        });
    }

    // Mode endpoints
    async get_mode() {
        return this.call({
            method: 'get',
            path: '/api/mode',
        });
    }

    async toggle_mode(mode, lan_cidr) {
        return this.call({
            method: 'patch',
            path: '/api/mode/toggle',
            body: {
                mode: mode,
                lan_cidr: lan_cidr
            }
        });
    }

    async can_switch_mode() {
        return this.call({
            method: 'get',
            path: '/api/mode/can-switch',
        });
    }

    async update_peer_route_status(prefix, active_peer_id, backup_peer_ids, timeout = 30000) {
        // Create AbortController for timeout
        const controller = new AbortController();
        const timeoutId = setTimeout(() => controller.abort(), timeout);
        
        try {
            const result = await this.call({
                method: 'patch',
                path: '/api/mode/peer-route-status',
                body: {
                    prefix: prefix,
                    active_peer_id: active_peer_id,
                    backup_peer_ids: backup_peer_ids
                },
                signal: controller.signal
            });
            clearTimeout(timeoutId);
            return result;
        } catch (error) {
            clearTimeout(timeoutId);
            if (error.name === 'AbortError') {
                throw new Error(`Request timed out after ${timeout / 1000} seconds`);
            }
            throw error;
        }
    }

    async get_exit_node_info() {
        return this.call({
            method: 'get',
            path: '/api/mode/exit-node',
        });
    }

    async peer_control(peer_id, action) {
        return this.call({
            method: 'post',
            path: '/api/peer/control',
            body: {
                peer_id: peer_id,
                action: action
            }
        });
    }

    async get_peer_lan_access() {
        return this.call({
            method: 'get',
            path: '/api/peer/lan-access',
        });
    }

    async set_peer_lan_access(peer_id, has_lan_access) {
        return this.call({
            method: 'patch',
            path: '/api/peer/lan-access',
            body: {
                peer_id: peer_id,
                has_lan_access: has_lan_access
            }
        });
    }

    async restore_routing_table() {
        return this.call({
            method: 'post',
            path: '/api/mode/restore-routing',
        });
    }

    // Init endpoints
    async get_init_status() {
        return this.call({
            method: 'get',
            path: '/api/init/status',
        });
    }

    async get_init_info() {
        return this.call({
            method: 'get',
            path: '/api/init/info',
        });
    }

    async post_init(initData) {
        return this.call({
            method: 'post',
            path: '/api/init',
            body: initData
        });
    }

}