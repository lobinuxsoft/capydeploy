<script lang="ts">
	import { browser } from '$app/environment';
	import { Card, Badge, Button, Input } from '$lib/components/ui';
	import { GetStatus, GetVersion, SetAcceptConnections, DisconnectHub, SetName, GetInstallPath, SelectInstallPath, SetTelemetryEnabled, SetTelemetryInterval, EventsOn, EventsOff } from '$lib/wailsjs';
	import type { AgentStatus, VersionInfo } from '$lib/types';
	import { Monitor, Wifi, WifiOff, Unplug, Pencil, Check, X, Folder, FolderOpen, Key, Info, ChevronDown, ChevronRight, Activity } from 'lucide-svelte';

	let status = $state<AgentStatus | null>(null);
	let versionInfo = $state<VersionInfo | null>(null);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let editingName = $state(false);
	let newName = $state('');
	let savingName = $state(false);
	let installPath = $state('');
	let pairingCode = $state<string | null>(null);
	let pairingTimer: ReturnType<typeof setTimeout> | null = null;

	// Collapsible sections state
	let expandedSections = $state<Set<string>>(new Set(['version', 'install', 'network', 'telemetry', 'connections']));

	function toggleSection(section: string) {
		if (expandedSections.has(section)) {
			expandedSections.delete(section);
		} else {
			expandedSections.add(section);
		}
		expandedSections = new Set(expandedSections);
	}

	async function loadStatus() {
		try {
			status = await GetStatus();
			installPath = await GetInstallPath();
			versionInfo = await GetVersion();
			error = null;
		} catch (e) {
			error = e instanceof Error ? e.message : 'Error loading status';
		} finally {
			loading = false;
		}
	}

	async function selectFolder() {
		try {
			const path = await SelectInstallPath();
			if (path) {
				installPath = path;
			}
		} catch (e) {
			console.error('Error selecting folder:', e);
		}
	}

	async function toggleConnections() {
		if (!status) return;
		await SetAcceptConnections(!status.acceptConnections);
	}

	async function disconnect() {
		await DisconnectHub();
	}

	function startEditName() {
		if (status) {
			newName = status.name;
			editingName = true;
		}
	}

	function cancelEditName() {
		editingName = false;
		newName = '';
	}

	async function saveName() {
		if (!newName.trim()) return;

		savingName = true;
		try {
			await SetName(newName.trim());
			editingName = false;
		} catch (e) {
			console.error('Failed to save name:', e);
		} finally {
			savingName = false;
		}
	}

	$effect(() => {
		if (!browser) return;

		loadStatus();

		EventsOn('server:started', (data: AgentStatus) => {
			status = data;
		});

		EventsOn('status:changed', (data: AgentStatus) => {
			status = data;
		});

		EventsOn('server:error', (err: string) => {
			error = err;
		});

		EventsOn('pairing:code', (code: string) => {
			pairingCode = code;
			// Clear existing timer
			if (pairingTimer) {
				clearTimeout(pairingTimer);
			}
			// Auto-hide after 60 seconds
			pairingTimer = setTimeout(() => {
				pairingCode = null;
			}, 60000);
		});

		EventsOn('pairing:success', () => {
			pairingCode = null;
			if (pairingTimer) {
				clearTimeout(pairingTimer);
				pairingTimer = null;
			}
		});

		return () => {
			EventsOff('server:started');
			EventsOff('status:changed');
			EventsOff('server:error');
			EventsOff('pairing:code');
			EventsOff('pairing:success');
			if (pairingTimer) {
				clearTimeout(pairingTimer);
			}
		};
	});

	async function toggleTelemetry() {
		if (!status) return;
		await SetTelemetryEnabled(!status.telemetryEnabled);
	}

	async function changeTelemetryInterval(event: Event) {
		const target = event.target as HTMLSelectElement;
		const seconds = parseInt(target.value, 10);
		if (!isNaN(seconds)) {
			await SetTelemetryInterval(seconds);
		}
	}

	function getPlatformIcon(platform: string) {
		switch (platform.toLowerCase()) {
			case 'windows':
				return 'Windows';
			case 'linux':
				return 'Linux';
			case 'darwin':
				return 'macOS';
			default:
				return platform;
		}
	}
</script>

<div class="space-y-4">
	{#if loading}
		<div class="cd-section p-8 flex items-center justify-center">
			<div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"></div>
		</div>
	{:else if error}
		<div class="cd-section p-8 text-center">
			<p class="cd-text-destructive">{error}</p>
			<Button variant="outline" class="mt-4" onclick={loadStatus}>
				Retry
			</Button>
		</div>
	{:else if status}
		<!-- Name -->
		<div class="cd-section p-4">
			<div class="flex items-center justify-between">
				<div class="flex items-center gap-2">
					<Monitor class="w-4 h-4 cd-text-disabled" />
					<span class="cd-section-title">Name</span>
				</div>
				{#if !editingName}
					<button
						type="button"
						class="flex items-center gap-2 hover:text-primary transition-colors"
						onclick={startEditName}
					>
						<span class="cd-value font-medium">{status.name}</span>
						<Pencil class="w-3 h-3 cd-text-disabled" />
					</button>
				{/if}
			</div>
			{#if editingName}
				<div class="flex items-center gap-2 mt-2">
					<Input
						bind:value={newName}
						placeholder="Agent name"
						class="flex-1"
						disabled={savingName}
					/>
					<Button
						size="icon"
						variant="ghost"
						onclick={saveName}
						disabled={savingName || !newName.trim()}
					>
						<Check class="w-4 h-4 text-success" />
					</Button>
					<Button
						size="icon"
						variant="ghost"
						onclick={cancelEditName}
						disabled={savingName}
					>
						<X class="w-4 h-4 text-destructive" />
					</Button>
				</div>
			{/if}
		</div>

		<div class="cd-section p-4 flex items-center justify-between">
			<span class="cd-section-title">Platform</span>
			<span class="cd-value font-medium">{getPlatformIcon(status.platform)}</span>
		</div>

		<!-- Version Info -->
		<div class="cd-section p-4">
			<button
				type="button"
				class="w-full flex items-center gap-2 hover:text-primary transition-colors"
				onclick={() => toggleSection('version')}
			>
				{#if expandedSections.has('version')}
					<ChevronDown class="w-4 h-4 cd-text-primary" />
				{:else}
					<ChevronRight class="w-4 h-4 cd-text-disabled" />
				{/if}
				<Info class="w-4 h-4 cd-text-disabled" />
				<span class="cd-section-title">Version</span>
			</button>
			{#if expandedSections.has('version')}
				<div class="pl-6 space-y-1 mt-2">
					<div class="flex justify-between text-sm">
						<span class="cd-text-disabled">Version</span>
						<span class="cd-mono">{versionInfo?.version ?? status.version}</span>
					</div>
					{#if versionInfo?.commit && versionInfo.commit !== 'unknown'}
						<div class="flex justify-between text-sm">
							<span class="cd-text-disabled">Commit</span>
							<span class="cd-mono text-xs">{versionInfo.commit}</span>
						</div>
					{/if}
					{#if versionInfo?.buildDate && versionInfo.buildDate !== 'unknown'}
						<div class="flex justify-between text-sm">
							<span class="cd-text-disabled">Build</span>
							<span class="cd-mono text-xs">{versionInfo.buildDate}</span>
						</div>
					{/if}
				</div>
			{/if}
		</div>

		<!-- Install Path -->
		<div class="cd-section p-4">
			<div class="flex items-center justify-between">
				<button
					type="button"
					class="flex items-center gap-2 hover:text-primary transition-colors"
					onclick={() => toggleSection('install')}
				>
					{#if expandedSections.has('install')}
						<ChevronDown class="w-4 h-4 cd-text-primary" />
					{:else}
						<ChevronRight class="w-4 h-4 cd-text-disabled" />
					{/if}
					<Folder class="w-4 h-4 cd-text-disabled" />
					<span class="cd-section-title">Install path</span>
				</button>
				<button
					type="button"
					class="p-1 hover:bg-secondary rounded transition-colors"
					onclick={selectFolder}
					title="Change folder"
				>
					<FolderOpen class="w-4 h-4 cd-text-disabled hover:text-primary" />
				</button>
			</div>
			{#if expandedSections.has('install')}
				<p class="cd-mono text-xs mt-2 pl-6 break-all">
					{installPath || '~/Games'}
				</p>
			{/if}
		</div>

		<!-- Network Info -->
		<div class="cd-section p-4">
			<button
				type="button"
				class="w-full flex items-center gap-2 hover:text-primary transition-colors"
				onclick={() => toggleSection('network')}
			>
				{#if expandedSections.has('network')}
					<ChevronDown class="w-4 h-4 cd-text-primary" />
				{:else}
					<ChevronRight class="w-4 h-4 cd-text-disabled" />
				{/if}
				<Wifi class="w-4 h-4 cd-text-disabled" />
				<span class="cd-section-title">Network</span>
			</button>
			{#if expandedSections.has('network')}
				<div class="pl-6 space-y-1 mt-2">
					<div class="flex justify-between text-sm">
						<span class="cd-text-disabled">Port</span>
						<span class="cd-mono">{status.port}</span>
					</div>
					{#each status.ips as ip}
						<div class="flex justify-between text-sm">
							<span class="cd-text-disabled">IP</span>
							<span class="cd-mono">{ip}</span>
						</div>
					{/each}
				</div>
			{/if}
		</div>

		<!-- Telemetry -->
		<div class="cd-section p-4">
			<div class="flex items-center justify-between">
				<button
					type="button"
					class="flex items-center gap-2 hover:text-primary transition-colors"
					onclick={() => toggleSection('telemetry')}
				>
					{#if expandedSections.has('telemetry')}
						<ChevronDown class="w-4 h-4 cd-text-primary" />
					{:else}
						<ChevronRight class="w-4 h-4 cd-text-disabled" />
					{/if}
					<Activity class="w-4 h-4 cd-text-disabled" />
					<span class="cd-section-title">Telemetry</span>
				</button>
				<Badge variant={status.telemetryEnabled ? 'success' : 'warning'}>
					{status.telemetryEnabled ? 'Sending' : 'Off'}
				</Badge>
			</div>

			{#if expandedSections.has('telemetry')}
				<div class="mt-3">
					<div class="flex items-center justify-between mb-3">
						<span class="text-sm cd-text-disabled">Interval</span>
						<select
							class="cd-select text-sm"
							value={status.telemetryInterval}
							onchange={changeTelemetryInterval}
						>
							<option value={1}>1s</option>
							<option value={2}>2s</option>
							<option value={3}>3s</option>
							<option value={5}>5s</option>
							<option value={10}>10s</option>
						</select>
					</div>
					<Button
						variant={status.telemetryEnabled ? 'destructive' : 'gradient'}
						class="w-full"
						onclick={toggleTelemetry}
					>
						{status.telemetryEnabled ? 'Stop Sending' : 'Enable Sending'}
					</Button>
				</div>
			{/if}
		</div>

		<!-- Pairing Code (shown when a Hub requests pairing) -->
		{#if pairingCode}
			<div class="cd-section p-4">
				<div class="flex items-center gap-2 mb-3">
					<span class="cd-pulse"></span>
					<Key class="w-5 h-5 cd-text-primary" />
					<span class="cd-section-title">Pairing Code</span>
				</div>
				<div class="text-center">
					<p class="cd-pairing-code">
						{pairingCode}
					</p>
					<p class="text-xs cd-text-disabled mt-3">
						Enter this code in the Hub to authorize the connection
					</p>
				</div>
			</div>
		{/if}

		<!-- Connection Status -->
		<div class="cd-section p-4">
			<div class="flex items-center justify-between">
				<button
					type="button"
					class="flex items-center gap-2 hover:text-primary transition-colors"
					onclick={() => toggleSection('connections')}
				>
					{#if expandedSections.has('connections')}
						<ChevronDown class="w-4 h-4 cd-text-primary" />
					{:else}
						<ChevronRight class="w-4 h-4 cd-text-disabled" />
					{/if}
					{#if status.acceptConnections}
						<Wifi class="w-4 h-4 cd-text-primary" />
					{:else}
						<WifiOff class="w-4 h-4 cd-text-destructive" />
					{/if}
					<span class="cd-section-title">Connections</span>
				</button>
				<Badge variant={status.acceptConnections ? 'success' : 'warning'}>
					{status.acceptConnections ? 'Accepting' : 'Blocked'}
				</Badge>
			</div>

			{#if expandedSections.has('connections')}
				<div class="mt-3">
					{#if status.connectedHub}
						<div class="flex items-center gap-2 p-3 mb-3 rounded-lg bg-primary/10 border border-primary/30">
							<span class="cd-pulse"></span>
							<Monitor class="w-4 h-4 cd-text-primary" />
							<span class="cd-status-connected">{status.connectedHub.name}</span>
							<span class="text-xs cd-text-disabled">({status.connectedHub.ip})</span>
						</div>
					{:else if !status.acceptConnections}
						<p class="text-xs cd-text-disabled mb-3">
							The Hub can see this agent but cannot perform operations
						</p>
					{/if}

					<Button
						variant={status.acceptConnections ? 'destructive' : 'gradient'}
						class="w-full"
						onclick={toggleConnections}
					>
						{status.acceptConnections ? 'Block Operations' : 'Allow Operations'}
					</Button>
				</div>
			{/if}
		</div>
	{/if}
</div>
