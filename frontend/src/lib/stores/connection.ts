import { writable } from 'svelte/store';
import type { ConnectionStatus } from '$lib/types';

function createConnectionStore() {
	const { subscribe, set, update } = writable<ConnectionStatus>({
		connected: false,
		deviceName: '',
		host: '',
		port: 0
	});

	return {
		subscribe,
		set,
		update,
		reset: () => set({
			connected: false,
			deviceName: '',
			host: '',
			port: 0
		})
	};
}

export const connectionStatus = createConnectionStore();
