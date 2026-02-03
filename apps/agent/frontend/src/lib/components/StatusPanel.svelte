<script lang="ts">
	import { browser } from '$app/environment';
	import { Card, Badge, Button, Input } from '$lib/components/ui';
	import { GetStatus, SetAcceptConnections, DisconnectHub, SetName, GetInstallPath, SelectInstallPath, EventsOn, EventsOff } from '$lib/wailsjs';
	import type { AgentStatus } from '$lib/types';
	import { Monitor, Wifi, WifiOff, Unplug, Pencil, Check, X, Folder, FolderOpen, Key } from 'lucide-svelte';

	let status = $state<AgentStatus | null>(null);
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

		return () => {
			EventsOff('server:started');
			EventsOff('status:changed');
			EventsOff('server:error');
			EventsOff('pairing:code');
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

<Card class="p-6">
	{#if loading}
		<div class="flex items-center justify-center py-8">
			<div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"></div>
		</div>
	{:else if error}
		<div class="text-center py-8 text-destructive">
			<p>{error}</p>
			<Button variant="outline" class="mt-4" onclick={loadStatus}>
				Reintentar
			</Button>
		</div>
	{:else if status}
		<div class="space-y-4">
			<!-- Name -->
			<div class="p-3 rounded-lg bg-secondary/50">
				<div class="flex items-center justify-between">
					<div class="flex items-center gap-2">
						<Monitor class="w-4 h-4" />
						<span class="text-sm">Nombre</span>
					</div>
					{#if !editingName}
						<button
							type="button"
							class="flex items-center gap-2 hover:text-primary transition-colors"
							onclick={startEditName}
						>
							<span class="text-sm font-medium">{status.name}</span>
							<Pencil class="w-3 h-3 text-muted-foreground" />
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

			<div class="flex items-center justify-between p-3 rounded-lg bg-secondary/50">
				<span class="text-sm">Plataforma</span>
				<span class="text-sm font-medium">{getPlatformIcon(status.platform)}</span>
			</div>

			<div class="flex items-center justify-between p-3 rounded-lg bg-secondary/50">
				<span class="text-sm">Version</span>
				<span class="text-sm font-mono">{status.version}</span>
			</div>

			<!-- Install Path -->
			<div class="p-3 rounded-lg bg-secondary/50">
				<div class="flex items-center justify-between">
					<div class="flex items-center gap-2">
						<Folder class="w-4 h-4" />
						<span class="text-sm">Ruta de instalación</span>
					</div>
					<button
						type="button"
						class="p-1 hover:bg-secondary rounded transition-colors"
						onclick={selectFolder}
						title="Cambiar carpeta"
					>
						<FolderOpen class="w-4 h-4 text-muted-foreground hover:text-primary" />
					</button>
				</div>
				<p class="text-xs font-mono text-muted-foreground mt-2 pl-6 break-all">
					{installPath || '~/Games'}
				</p>
			</div>

			<!-- Network Info -->
			<div class="p-3 rounded-lg bg-secondary/50">
				<div class="flex items-center gap-2 mb-2">
					<Wifi class="w-4 h-4" />
					<span class="text-sm font-medium">Red</span>
				</div>
				<div class="pl-6 space-y-1">
					<div class="flex justify-between text-sm">
						<span class="text-muted-foreground">Puerto</span>
						<span class="font-mono">{status.port}</span>
					</div>
					{#each status.ips as ip}
						<div class="flex justify-between text-sm">
							<span class="text-muted-foreground">IP</span>
							<span class="font-mono">{ip}</span>
						</div>
					{/each}
				</div>
			</div>

			<!-- Pairing Code (shown when a Hub requests pairing) -->
			{#if pairingCode}
				<div class="p-4 rounded-lg bg-primary/10 border border-primary/30 animate-pulse">
					<div class="flex items-center gap-2 mb-2">
						<Key class="w-5 h-5 text-primary" />
						<span class="text-sm font-medium text-primary">Código de Emparejamiento</span>
					</div>
					<div class="text-center">
						<p class="text-3xl font-mono font-bold tracking-[0.5em] text-primary">
							{pairingCode}
						</p>
						<p class="text-xs text-muted-foreground mt-2">
							Ingresa este código en el Hub para autorizar la conexión
						</p>
					</div>
				</div>
			{/if}

			<!-- Connection Status -->
			<div class="p-3 rounded-lg bg-secondary/50">
				<div class="flex items-center justify-between mb-3">
					<div class="flex items-center gap-2">
						{#if status.acceptConnections}
							<Wifi class="w-4 h-4 text-success" />
						{:else}
							<WifiOff class="w-4 h-4 text-destructive" />
						{/if}
						<span class="text-sm font-medium">Conexiones</span>
					</div>
					<Badge variant={status.acceptConnections ? 'success' : 'warning'}>
						{status.acceptConnections ? 'Aceptando' : 'Bloqueadas'}
					</Badge>
				</div>

				{#if !status.acceptConnections}
					<p class="text-xs text-muted-foreground mb-3">
						El Hub puede ver este agente pero no puede realizar operaciones
					</p>
				{/if}

				<Button
					variant={status.acceptConnections ? 'destructive' : 'default'}
					class="w-full"
					onclick={toggleConnections}
				>
					{status.acceptConnections ? 'Bloquear Operaciones' : 'Permitir Operaciones'}
				</Button>
			</div>
		</div>
	{/if}
</Card>
