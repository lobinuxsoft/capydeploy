<script lang="ts">
	import { browser } from '$app/environment';
	import { Card, Badge, Button, Input } from '$lib/components/ui';
	import { GetStatus, GetVersion, GetCapabilities, SetAcceptConnections, DisconnectHub, SetName, GetInstallPath, SelectInstallPath, EventsOn, EventsOff } from '$lib/wailsjs';
	import type { AgentStatus, VersionInfo, CapabilityInfo } from '$lib/types';
	import { Monitor, Wifi, WifiOff, Unplug, Pencil, Check, X, Folder, FolderOpen, Key, Info, Zap } from 'lucide-svelte';

	let status = $state<AgentStatus | null>(null);
	let versionInfo = $state<VersionInfo | null>(null);
	let capabilities = $state<CapabilityInfo[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let editingName = $state(false);
	let newName = $state('');
	let savingName = $state(false);
	let installPath = $state('');
	let pairingCode = $state<string | null>(null);
	let pairingTimer: ReturnType<typeof setTimeout> | null = null;

	async function loadStatus() {
		try {
			status = await GetStatus();
			installPath = await GetInstallPath();
			versionInfo = await GetVersion();
			capabilities = await GetCapabilities();
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
				Reintentar
			</Button>
		</div>
	{:else if status}
		<!-- Name -->
		<div class="cd-section p-4">
			<div class="flex items-center justify-between">
				<div class="flex items-center gap-2">
					<Monitor class="w-4 h-4 cd-text-disabled" />
					<span class="text-sm">Nombre</span>
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
						placeholder="Nombre del agente"
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
			<span class="text-sm">Plataforma</span>
			<span class="cd-value font-medium">{getPlatformIcon(status.platform)}</span>
		</div>

		<!-- Version Info -->
		<div class="cd-section p-4">
			<div class="flex items-center gap-2 mb-2">
				<Info class="w-4 h-4 cd-text-disabled" />
				<span class="cd-section-title">Version</span>
			</div>
			<div class="pl-6 space-y-1">
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
		</div>

		<!-- Install Path -->
		<div class="cd-section p-4">
			<div class="flex items-center justify-between">
				<div class="flex items-center gap-2">
					<Folder class="w-4 h-4 cd-text-disabled" />
					<span class="text-sm">Ruta de instalación</span>
				</div>
				<button
					type="button"
					class="p-1 hover:bg-secondary rounded transition-colors"
					onclick={selectFolder}
					title="Cambiar carpeta"
				>
					<FolderOpen class="w-4 h-4 cd-text-disabled hover:text-primary" />
				</button>
			</div>
			<p class="cd-mono text-xs mt-2 pl-6 break-all">
				{installPath || '~/Games'}
			</p>
		</div>

		<!-- Network Info -->
		<div class="cd-section p-4">
			<div class="flex items-center gap-2 mb-2">
				<Wifi class="w-4 h-4 cd-text-disabled" />
				<span class="cd-section-title">Red</span>
			</div>
			<div class="pl-6 space-y-1">
				<div class="flex justify-between text-sm">
					<span class="cd-text-disabled">Puerto</span>
					<span class="cd-mono">{status.port}</span>
				</div>
				{#each status.ips as ip}
					<div class="flex justify-between text-sm">
						<span class="cd-text-disabled">IP</span>
						<span class="cd-mono">{ip}</span>
					</div>
				{/each}
			</div>
		</div>

		<!-- Capabilities -->
		{#if capabilities.length > 0}
			<div class="cd-section p-4">
				<div class="flex items-center gap-2 mb-3">
					<Zap class="w-4 h-4 cd-text-primary" />
					<span class="cd-section-title">Capacidades</span>
				</div>
				<div class="pl-6 space-y-2">
					{#each capabilities as cap}
						<div class="flex items-start gap-2">
							<span class="cd-pulse mt-1" style="width: 6px; height: 6px;"></span>
							<div>
								<span class="text-sm font-medium">{cap.name}</span>
								<p class="text-xs cd-text-disabled">{cap.description}</p>
							</div>
						</div>
					{/each}
				</div>
			</div>
		{/if}

		<!-- Pairing Code (shown when a Hub requests pairing) -->
		{#if pairingCode}
			<div class="cd-section p-4">
				<div class="flex items-center gap-2 mb-3">
					<span class="cd-pulse"></span>
					<Key class="w-5 h-5 cd-text-primary" />
					<span class="cd-section-title">Código de Emparejamiento</span>
				</div>
				<div class="text-center">
					<p class="cd-pairing-code">
						{pairingCode}
					</p>
					<p class="text-xs cd-text-disabled mt-3">
						Ingresa este código en el Hub para autorizar la conexión
					</p>
				</div>
			</div>
		{/if}

		<!-- Connection Status -->
		<div class="cd-section p-4">
			<div class="flex items-center justify-between mb-3">
				<div class="flex items-center gap-2">
					{#if status.acceptConnections}
						<Wifi class="w-4 h-4 cd-text-primary" />
					{:else}
						<WifiOff class="w-4 h-4 cd-text-destructive" />
					{/if}
					<span class="cd-section-title">Conexiones</span>
				</div>
				<Badge variant={status.acceptConnections ? 'success' : 'warning'}>
					{status.acceptConnections ? 'Aceptando' : 'Bloqueadas'}
				</Badge>
			</div>

			{#if status.connectedHub}
				<div class="flex items-center gap-2 p-3 mb-3 rounded-lg bg-primary/10 border border-primary/30">
					<span class="cd-pulse"></span>
					<Monitor class="w-4 h-4 cd-text-primary" />
					<span class="cd-status-connected">{status.connectedHub.name}</span>
					<span class="text-xs cd-text-disabled">({status.connectedHub.ip})</span>
				</div>
			{:else if !status.acceptConnections}
				<p class="text-xs cd-text-disabled mb-3">
					El Hub puede ver este agente pero no puede realizar operaciones
				</p>
			{/if}

			<Button
				variant={status.acceptConnections ? 'destructive' : 'gradient'}
				class="w-full"
				onclick={toggleConnections}
			>
				{status.acceptConnections ? 'Bloquear Operaciones' : 'Permitir Operaciones'}
			</Button>
		</div>
	{/if}
</div>
