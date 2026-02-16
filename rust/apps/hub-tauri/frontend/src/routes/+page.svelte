<script lang="ts">
	import { Tabs } from '$lib/components/ui';
	import { ConnectionStatus, DeviceList, GameSetupList, InstalledGames, Settings, Telemetry, ConsoleLog } from '$lib/components';
	import { connectionStatus } from '$lib/stores/connection';
	import { telemetry } from '$lib/stores/telemetry';
	import { consolelog } from '$lib/stores/consolelog';
	import { EventsOn } from '$lib/wailsjs';
	import { browser } from '$app/environment';

	// Tabs are dynamic based on connection status.
	// "Upload Game" and "Installed Games" only appear when an agent is connected.
	let tabs = $derived([
		{ id: 'devices', label: 'Devices' },
		...($connectionStatus.connected ? [
			{ id: 'upload', label: 'Upload Game' },
			{ id: 'games', label: 'Installed Games' },
			{ id: 'telemetry', label: 'Telemetry' },
			{ id: 'consolelog', label: 'Console' }
		] : []),
		{ id: 'settings', label: 'Settings' }
	]);

	// Listen for connection status changes
	$effect(() => {
		if (!browser) return;

		const unsub = EventsOn('connection:changed', (status) => {
			connectionStatus.set(status);
			if (!status.connected) {
				telemetry.reset();
				consolelog.reset();
			}
		});

		return unsub;
	});
</script>

<div class="min-h-screen text-foreground">
	<!-- Header with mascot and connection status (Decky style) -->
	<div class="m-4 mb-0">
		<div class="cd-header">
			<div class="cd-mascot-wrap">
				<img src="/mascot.webp" alt="CapyDeploy" />
			</div>
			<div class="flex-1">
				<h1 class="cd-title">CapyDeploy Hub</h1>
				<p class="cd-subtitle">Manage and deploy games to your devices</p>
			</div>
			<ConnectionStatus />
		</div>
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
				{:else if activeTab === 'telemetry'}
					<Telemetry />
				{:else if activeTab === 'consolelog'}
					<ConsoleLog />
				{:else if activeTab === 'settings'}
					<Settings />
				{/if}
			{/snippet}
		</Tabs>
	</div>
</div>
