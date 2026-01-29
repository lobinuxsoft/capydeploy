import { writable } from 'svelte/store';
import type { DeviceConfig } from '$lib/types';

function createDevicesStore() {
	const { subscribe, set, update } = writable<DeviceConfig[]>([]);

	return {
		subscribe,
		set,
		update,
		add: (device: DeviceConfig) => update(devices => [...devices, device]),
		remove: (host: string) => update(devices => devices.filter(d => d.host !== host)),
		updateDevice: (oldHost: string, device: DeviceConfig) => update(devices =>
			devices.map(d => d.host === oldHost ? device : d)
		)
	};
}

export const devices = createDevicesStore();
