<script lang="ts">
	import { ConsoleLog } from '$lib/components';
	import { consolelog } from '$lib/stores/consolelog';
	import { EventsOn } from '$lib/wailsjs';
	import { browser } from '$app/environment';
	import { getCurrentWindow } from '@tauri-apps/api/window';
	import { RefreshCw } from 'lucide-svelte';
	import { requestStateSync } from '$lib/popout';
	import type { ConsoleLogStatus, ConsoleLogBatch, ConnectionStatus } from '$lib/types';

	function refresh() {
		consolelog.clear();
		requestStateSync();
	}

	$effect(() => {
		if (!browser) return;

		// Request current state from main window on mount
		requestStateSync();

		const unsubConnection = EventsOn('connection:changed', (status: ConnectionStatus) => {
			if (!status.connected) {
				getCurrentWindow().close();
			}
		});

		const unsubClStatus = EventsOn('consolelog:status', (event: ConsoleLogStatus) => {
			consolelog.status.set(event);
		});

		const unsubClData = EventsOn('consolelog:data', (event: ConsoleLogBatch) => {
			consolelog.addBatch(event.entries, event.dropped);
		});

		return () => {
			unsubConnection();
			unsubClStatus();
			unsubClData();
		};
	});
</script>

<div class="min-h-screen text-foreground p-4">
	<div class="flex items-center justify-between mb-4">
		<h2 class="text-sm font-medium text-muted-foreground">Console Log</h2>
		<button
			type="button"
			onclick={refresh}
			class="p-1.5 rounded hover:bg-secondary transition-colors text-muted-foreground hover:text-foreground"
			title="Refresh data"
		>
			<RefreshCw class="w-4 h-4" />
		</button>
	</div>
	<ConsoleLog />
</div>
