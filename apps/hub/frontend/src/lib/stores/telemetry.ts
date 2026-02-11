import { writable } from 'svelte/store';
import type { TelemetryStatus, TelemetryData } from '$lib/types';

function createTelemetryStore() {
	const status = writable<TelemetryStatus>({ enabled: false, interval: 2 });
	const data = writable<TelemetryData | null>(null);

	return {
		status,
		data,
		reset: () => {
			status.set({ enabled: false, interval: 2 });
			data.set(null);
		}
	};
}

export const telemetry = createTelemetryStore();
