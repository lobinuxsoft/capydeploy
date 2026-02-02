import { writable } from 'svelte/store';
import type { ConnectionStatus } from '$lib/types';

function createConnectionStore() {
	const { subscribe, set, update } = writable<ConnectionStatus>({
		connected: false,
		agentId: '',
		agentName: '',
		platform: '',
		host: '',
		port: 0,
		ips: [],
		supportedImageFormats: []
	});

	return {
		subscribe,
		set,
		update,
		reset: () => set({
			connected: false,
			agentId: '',
			agentName: '',
			platform: '',
			host: '',
			port: 0,
			ips: [],
			supportedImageFormats: []
		})
	};
}

export const connectionStatus = createConnectionStore();
