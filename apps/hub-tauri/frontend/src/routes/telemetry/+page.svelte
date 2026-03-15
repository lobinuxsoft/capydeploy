<script lang="ts">
	import { Telemetry } from '$lib/components';
	import { telemetry } from '$lib/stores/telemetry';
	import { EventsOn } from '$lib/wailsjs';
	import { browser } from '$app/environment';
	import { getCurrentWindow } from '@tauri-apps/api/window';
	import { RefreshCw } from 'lucide-svelte';
	import { requestStateSync } from '$lib/popout';
	import type { TelemetryStatus, TelemetryData, ConnectionStatus } from '$lib/types';

	$effect(() => {
		if (!browser) return;

		// Request current state from main window on mount
		requestStateSync();

		const unsubConnection = EventsOn('connection:changed', (status: ConnectionStatus) => {
			if (!status.connected) {
				getCurrentWindow().close();
			}
		});

		const unsubTelStatus = EventsOn('telemetry:status', (event: TelemetryStatus) => {
			telemetry.status.set(event);
		});

		const unsubTelData = EventsOn('telemetry:data', (event: TelemetryData) => {
			telemetry.data.set(event);
		});

		return () => {
			unsubConnection();
			unsubTelStatus();
			unsubTelData();
		};
	});
</script>

<div class="min-h-screen text-foreground p-4">
	<div class="flex items-center justify-between mb-4">
		<h2 class="text-sm font-medium text-muted-foreground">Telemetry</h2>
		<button
			type="button"
			onclick={() => requestStateSync()}
			class="p-1.5 rounded hover:bg-secondary transition-colors text-muted-foreground hover:text-foreground"
			title="Refresh data"
		>
			<RefreshCw class="w-4 h-4" />
		</button>
	</div>
	<Telemetry />
</div>
