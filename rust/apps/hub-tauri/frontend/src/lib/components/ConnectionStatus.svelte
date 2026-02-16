<script lang="ts">
	import { connectionStatus } from '$lib/stores/connection';

	let status = $derived($connectionStatus);

	function getPlatformIcon(platform: string): string {
		switch (platform?.toLowerCase()) {
			case 'linux': return '🐧';
			case 'windows': return '🪟';
			default: return '💻';
		}
	}
</script>

<div class="flex items-center gap-2 text-sm">
	{#if status.connected}
		<span class="cd-pulse"></span>
		<span class="cd-status-connected">
			{getPlatformIcon(status.platform)} {status.agentName}
			<span class="text-xs font-normal opacity-70">({status.ips?.[0] || status.host}:{status.port})</span>
		</span>
	{:else}
		<div class="w-2 h-2 rounded-full bg-muted-foreground/50"></div>
		<span class="cd-status-disconnected opacity-70">Not connected</span>
	{/if}
</div>
