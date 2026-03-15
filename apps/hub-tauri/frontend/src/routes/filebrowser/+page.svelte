<script lang="ts">
	import { FileBrowser } from '$lib/components';
	import { EventsOn } from '$lib/wailsjs';
	import { browser } from '$app/environment';
	import { getCurrentWindow } from '@tauri-apps/api/window';
	import type { ConnectionStatus } from '$lib/types';

	$effect(() => {
		if (!browser) return;

		const unsubConnection = EventsOn('connection:changed', (status: ConnectionStatus) => {
			if (!status.connected) {
				getCurrentWindow().close();
			}
		});

		return () => {
			unsubConnection();
		};
	});
</script>

<div class="min-h-screen text-foreground p-4">
	<FileBrowser />
</div>
