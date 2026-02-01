<script lang="ts">
	import { connectionStatus } from '$lib/stores/connection';
	import { cn } from '$lib/utils';

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
	<div
		class={cn(
			'w-2.5 h-2.5 rounded-full border border-gray-600',
			status.connected ? 'bg-green-500' : 'bg-gray-500'
		)}
	></div>
	<span class="text-muted-foreground italic">
		{#if status.connected}
			{getPlatformIcon(status.platform)} {status.agentName}
			<span class="text-xs">({status.ips?.[0] || status.host}:{status.port})</span>
		{:else}
			Not connected
		{/if}
	</span>
</div>
