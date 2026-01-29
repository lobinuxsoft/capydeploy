<script lang="ts">
	import { Tabs } from '$lib/components/ui';
	import { ConnectionStatus, DeviceList, GameSetupList, InstalledGames, Settings } from '$lib/components';
	import { connectionStatus } from '$lib/stores/connection';
	import { EventsOn, EventsOff } from '$lib/wailsjs';

	const tabs = [
		{ id: 'devices', label: 'Devices' },
		{ id: 'upload', label: 'Upload Game' },
		{ id: 'games', label: 'Installed Games' },
		{ id: 'settings', label: 'Settings' }
	];

	// Listen for connection status changes
	$effect(() => {
		EventsOn('connection:changed', (status) => {
			connectionStatus.set(status);
		});

		return () => {
			EventsOff('connection:changed');
		};
	});
</script>

<div class="min-h-screen bg-background text-foreground">
	<!-- Header with connection status -->
	<div class="flex items-center justify-end p-4 border-b">
		<ConnectionStatus />
	</div>

	<!-- Main content -->
	<div class="p-6">
		<Tabs {tabs}>
			{#snippet children(activeTab)}
				{#if activeTab === 'devices'}
					<DeviceList />
				{:else if activeTab === 'upload'}
					<GameSetupList />
				{:else if activeTab === 'games'}
					<InstalledGames />
				{:else if activeTab === 'settings'}
					<Settings />
				{/if}
			{/snippet}
		</Tabs>
	</div>
</div>
