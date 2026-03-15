<script lang="ts">
	import { Tabs } from '$lib/components/ui';
	import { ConnectionStatus, DeviceList, GameSetupList, InstalledGames, Settings } from '$lib/components';
	import { connectionStatus } from '$lib/stores/connection';
	import { telemetry } from '$lib/stores/telemetry';
	import { consolelog } from '$lib/stores/consolelog';
	import { EventsOn } from '$lib/wailsjs';
	import { browser } from '$app/environment';
	import type { TelemetryStatus, TelemetryData, ConsoleLogStatus, ConsoleLogBatch } from '$lib/types';
	import { openPopout, closeAllPopouts, POPOUT_LABELS } from '$lib/popout';
	import { emitTo } from '@tauri-apps/api/event';
	import { get } from 'svelte/store';
	import { Cpu, Terminal, FolderOpen } from 'lucide-svelte';

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

	// Global event listeners — must live here (always mounted) so events
	// are not lost when sub-components unmount on tab switch.
	$effect(() => {
		if (!browser) return;

		const unsubConnection = EventsOn('connection:changed', (status) => {
			connectionStatus.set(status);
			if (!status.connected) {
				telemetry.reset();
				consolelog.reset();
				closeAllPopouts();
			}
		});

		const unsubTelStatus = EventsOn('telemetry:status', (event: TelemetryStatus) => {
			telemetry.status.set(event);
		});

		const unsubTelData = EventsOn('telemetry:data', (event: TelemetryData) => {
			telemetry.data.set(event);
		});

		const unsubClStatus = EventsOn('consolelog:status', (event: ConsoleLogStatus) => {
			consolelog.status.set(event);
		});

		const unsubClData = EventsOn('consolelog:data', (event: ConsoleLogBatch) => {
			consolelog.addBatch(event.entries, event.dropped);
		});

		// Respond to pop-out windows requesting current state
		const unsubStateReq = EventsOn('popout:request-state', () => {
			const telStatus = get(telemetry.status);
			const telData = get(telemetry.data);
			const clStatus = get(consolelog.status);
			const clEntries = get(consolelog.entries);
			const clDropped = get(consolelog.totalDropped);

			for (const label of Object.values(POPOUT_LABELS)) {
				emitTo(label, 'telemetry:status', telStatus);
				if (telData) emitTo(label, 'telemetry:data', telData);
				emitTo(label, 'consolelog:status', clStatus);
				if (clEntries.length > 0) {
					emitTo(label, 'consolelog:data', { entries: clEntries, dropped: clDropped });
				}
			}
		});

		return () => {
			unsubConnection();
			unsubTelStatus();
			unsubTelData();
			unsubClStatus();
			unsubClData();
			unsubStateReq();
		};
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
		<div class="flex items-center gap-3 mb-4">
			<div class="flex-1">
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
			{#if $connectionStatus.connected}
				<div class="flex items-center gap-1 self-start">
					<button
						type="button"
						onclick={() => openPopout({ label: POPOUT_LABELS.telemetry, title: 'CapyDeploy - Telemetry', url: '/telemetry' })}
						class="inline-flex items-center gap-1.5 rounded-md px-3 py-1 text-sm font-medium text-muted-foreground hover:bg-background/50 hover:text-foreground transition-all"
						title="Open Telemetry window"
					>
						<Cpu class="w-4 h-4" />
						Telemetry
					</button>
					<button
						type="button"
						onclick={() => openPopout({ label: POPOUT_LABELS.consolelog, title: 'CapyDeploy - Console Log', url: '/consolelog' })}
						class="inline-flex items-center gap-1.5 rounded-md px-3 py-1 text-sm font-medium text-muted-foreground hover:bg-background/50 hover:text-foreground transition-all"
						title="Open Console Log window"
					>
						<Terminal class="w-4 h-4" />
						Console
					</button>
					{#if $connectionStatus.capabilities.includes('file_browser')}
						<button
							type="button"
							onclick={() => openPopout({ label: POPOUT_LABELS.filebrowser, title: 'CapyDeploy - Files', url: '/filebrowser', width: 900, height: 600 })}
							class="inline-flex items-center gap-1.5 rounded-md px-3 py-1 text-sm font-medium text-muted-foreground hover:bg-background/50 hover:text-foreground transition-all"
							title="Open File Browser window"
						>
							<FolderOpen class="w-4 h-4" />
							Files
						</button>
					{/if}
				</div>
			{/if}
		</div>
	</div>
</div>
