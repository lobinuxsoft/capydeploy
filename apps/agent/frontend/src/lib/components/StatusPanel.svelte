<script lang="ts">
	import { browser } from '$app/environment';
	import { Card, Badge, Button } from '$lib/components/ui';
	import { GetStatus, SetAcceptConnections, DisconnectHub, EventsOn, EventsOff } from '$lib/wailsjs';
	import type { AgentStatus } from '$lib/types';
	import { Monitor, Wifi, WifiOff, Server, Power, Unplug } from 'lucide-svelte';

	let status = $state<AgentStatus | null>(null);
	let loading = $state(true);
	let error = $state<string | null>(null);

	async function loadStatus() {
		try {
			status = await GetStatus();
			error = null;
		} catch (e) {
			error = e instanceof Error ? e.message : 'Error loading status';
		} finally {
			loading = false;
		}
	}

	async function toggleConnections() {
		if (!status) return;
		await SetAcceptConnections(!status.acceptConnections);
	}

	async function disconnect() {
		await DisconnectHub();
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

		return () => {
			EventsOff('server:started');
			EventsOff('status:changed');
			EventsOff('server:error');
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
	<div class="flex items-center gap-3 mb-6">
		<div class="p-2 rounded-lg bg-primary/10">
			<Server class="w-6 h-6 text-primary" />
		</div>
		<div>
			<h2 class="text-xl font-semibold">Agent Status</h2>
			<p class="text-sm text-muted-foreground">Estado del servidor</p>
		</div>
	</div>

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
			<!-- Server Status -->
			<div class="flex items-center justify-between p-3 rounded-lg bg-secondary/50">
				<div class="flex items-center gap-2">
					<Power class="w-4 h-4" />
					<span class="text-sm">Servidor</span>
				</div>
				<Badge variant={status.running ? 'success' : 'destructive'}>
					{status.running ? 'Activo' : 'Inactivo'}
				</Badge>
			</div>

			<!-- Name & Platform -->
			<div class="flex items-center justify-between p-3 rounded-lg bg-secondary/50">
				<div class="flex items-center gap-2">
					<Monitor class="w-4 h-4" />
					<span class="text-sm">Nombre</span>
				</div>
				<span class="text-sm font-medium">{status.name}</span>
			</div>

			<div class="flex items-center justify-between p-3 rounded-lg bg-secondary/50">
				<span class="text-sm">Plataforma</span>
				<span class="text-sm font-medium">{getPlatformIcon(status.platform)}</span>
			</div>

			<div class="flex items-center justify-between p-3 rounded-lg bg-secondary/50">
				<span class="text-sm">Version</span>
				<span class="text-sm font-mono">{status.version}</span>
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

				{#if status.connectedHub}
					<div class="p-2 rounded bg-success/10 border border-success/20 mb-3">
						<div class="flex items-center justify-between">
							<div>
								<p class="text-sm font-medium text-success">Conectado a Hub</p>
								<p class="text-xs text-muted-foreground">{status.connectedHub.name} ({status.connectedHub.ip})</p>
							</div>
							<Button variant="ghost" size="icon" onclick={disconnect}>
								<Unplug class="w-4 h-4" />
							</Button>
						</div>
					</div>
				{:else}
					<p class="text-sm text-muted-foreground mb-3">Sin conexion activa</p>
				{/if}

				<Button
					variant={status.acceptConnections ? 'destructive' : 'default'}
					class="w-full"
					onclick={toggleConnections}
				>
					{status.acceptConnections ? 'Bloquear Conexiones' : 'Aceptar Conexiones'}
				</Button>
			</div>
		</div>
	{/if}
</Card>
