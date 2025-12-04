'use strict';

import FastEqual from "fast-deep-equal";

export default class WireGuardHelper {

    static validateField(fieldName, validator, originalValue, island_change_sum, field_color_lookup, ...validatorArgs) {
        const result = validator(...validatorArgs);

        if (result.error) {
            island_change_sum.errors[fieldName] = result.error;
            return [field_color_lookup["error"], island_change_sum];
        }

        if (!FastEqual(result.value, originalValue)) {
            island_change_sum.changed_fields[fieldName] = result.value;
            return [field_color_lookup["changed"], island_change_sum];
        }

        island_change_sum.changed_fields[fieldName] = null;
        island_change_sum.errors[fieldName] = null;
        return [field_color_lookup["unchanged"], island_change_sum];
    }

    static get_field_colors(is_new) {
        return {
            unchanged: is_new ? 'enabled:bg-badge-success-bg' : '',
            changed: is_new ? 'enabled:bg-badge-success-bg' : 'enabled:bg-badge-info-bg',
            error: 'enabled:bg-badge-error-bg',
        }
    }

    static get_div_colors(is_new) {
        return {
            unchanged: is_new ? 'border-badge-success-border' : 'border-divider',
            changed: is_new ? 'border-badge-success-border' : 'border-badge-info-border',
            error: 'border-badge-error-border',
        }
    }

    static stringify_endpoint(endpoint) {
        if (endpoint.address === "none") {
            return "";
        }
        if ('ipv4_and_port' in endpoint.address) {
            return `${endpoint.address.ipv4_and_port.ipv4}:${endpoint.address.ipv4_and_port.port}`;
        }
        if ('hostname_and_port' in endpoint.address) {
            return `${endpoint.address.hostname_and_port.hostname}:${endpoint.address.hostname_and_port.port}`;
        }
        return "";
    }

}
