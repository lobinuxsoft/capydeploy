<script lang="ts">
	import { Tabs } from '$lib/components/ui';
	import { ConnectionStatus, DeviceList, GameSetupList, InstalledGames, Settings } from '$lib/components';
	import { connectionStatus } from '$lib/stores/connection';
	import { EventsOn, EventsOff } from '$lib/wailsjs';
	import { browser } from '$app/environment';

	// Tabs are dynamic based on connection status.
	// "Upload Game" and "Installed Games" only appear when an agent is connected.
	let tabs = $derived([
		{ id: 'devices', label: 'Devices' },
		...($connectionStatus.connected ? [
			{ id: 'upload', label: 'Upload Game' },
			{ id: 'games', label: 'Installed Games' }
		] : []),
		{ id: 'settings', label: 'Settings' }
	]);

	// Listen for connection status changes
	$effect(() => {
		if (!browser) return;

		EventsOn('connection:changed', (status) => {
			connectionStatus.set(status);
		});

		return () => {
			EventsOff('connection:changed');
		};
	});
</script>

<div class="min-h-screen text-foreground">
	<!-- Header with mascot and connection status -->
	<div class="flex items-center justify-between px-4 py-3 glass-card border-b border-border">
		<div class="flex items-center gap-3">
			<div class="mini-mascot">
				<img src="/mascot.gif" alt="CapyDeploy" />
			</div>
			<h1 class="text-lg font-bold gradient-text">CapyDeploy Hub</h1>
		</div>
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
