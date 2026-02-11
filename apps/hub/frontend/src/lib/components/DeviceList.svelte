<script lang="ts">
	import { Button, Card } from '$lib/components/ui';
	import PairingDialog from './PairingDialog.svelte';
	import { connectionStatus } from '$lib/stores/connection';
	import { toast } from '$lib/stores/toast';
	import type { DiscoveredAgent } from '$lib/types';
	import { Monitor, LogIn, LogOut, RefreshCw, Loader2, Wifi, WifiOff } from 'lucide-svelte';
	import { cn } from '$lib/utils';
	import {
		GetDiscoveredAgents, RefreshDiscovery, ConnectAgent, DisconnectAgent,
		GetConnectionStatus, EventsOn
	} from '$lib/wailsjs';
	import { browser } from '$app/environment';

	let agents = $state<DiscoveredAgent[]>([]);
	let connecting = $state<string | null>(null);
	let refreshing = $state(false);
	let showPairingDialog = $state(false);
	let pairingAgentName = $state('');

	async function loadAgents() {
		if (!browser) return;
		try {
			const list = await GetDiscoveredAgents();
			agents = list || [];
		} catch (e) {
			console.error('Failed to load agents:', e);
		}
	}

	async function loadConnectionStatus() {
		if (!browser) return;
		try {
			const status = await GetConnectionStatus();
			connectionStatus.set(status);
		} catch (e) {
			console.error('Failed to get connection status:', e);
		}
	}

	async function refresh() {
		if (!browser) return;
		refreshing = true;
		try {
			const list = await RefreshDiscovery();
			agents = list || [];
		} catch (e) {
			console.error('Failed to refresh:', e);
		} finally {
			refreshing = false;
		}
	}

	async function connect(agentID: string) {
		if (!browser) return;
		connecting = agentID;
		try {
			await ConnectAgent(agentID);
			await loadConnectionStatus();
			toast.success('Connected');
		} catch (e) {
			console.error('Failed to connect:', e);
			toast.error('Connection error', String(e));
		} finally {
			connecting = null;
		}
	}

	async function disconnect() {
		if (!browser) return;
		try {
			await DisconnectAgent();
			await loadConnectionStatus();
		} catch (e) {
			console.error('Failed to disconnect:', e);
		}
	}

	function getPlatformIcon(platform: string): string {
		switch (platform.toLowerCase()) {
			case 'linux': return '🐧';
			case 'windows': return '🪟';
			default: return '💻';
		}
	}

	function getPlatformLabel(platform: string): string {
		switch (platform.toLowerCase()) {
			case 'linux': return 'Linux';
			case 'windows': return 'Windows';
			default: return platform;
		}
	}

	// Initialize and setup event listeners
	$effect(() => {
		if (!browser) return;

		loadAgents();
		loadConnectionStatus();

		const unsubFound = EventsOn('discovery:agent-found', (agent: DiscoveredAgent) => {
			agents = [...agents.filter(a => a.id !== agent.id), agent];
		});

		const unsubUpdated = EventsOn('discovery:agent-updated', (agent: DiscoveredAgent) => {
			agents = agents.map(a => a.id === agent.id ? agent : a);
		});

		const unsubLost = EventsOn('discovery:agent-lost', (agentID: string) => {
			agents = agents.filter(a => a.id !== agentID);
		});

		const unsubConnection = EventsOn('connection:changed', (status: any) => {
			connectionStatus.set(status);
		});

		const unsubPairing = EventsOn('pairing:required', (agentID: string) => {
			const agent = agents.find(a => a.id === agentID);
			pairingAgentName = agent?.name || 'Agent';
			showPairingDialog = true;
			connecting = null;
		});

		return () => {
			unsubFound();
			unsubUpdated();
			unsubLost();
			unsubConnection();
			unsubPairing();
		};
	});

	function handlePairingSuccess() {
		loadConnectionStatus();
	}

	function handlePairingCancel() {
		connecting = null;
	}
</script>

<div class="space-y-4">
	<div class="flex items-center justify-between">
		<h3 class="cd-section-title">
			Discovered Agents ({agents.length})
		</h3>
		<Button variant="outline" size="sm" onclick={refresh} disabled={refreshing}>
			{#if refreshing}
				<Loader2 class="w-4 h-4 mr-2 animate-spin" />
			{:else}
				<RefreshCw class="w-4 h-4 mr-2" />
			{/if}
			Refresh
		</Button>
	</div>

	<div class="space-y-2">
		{#each agents as agent}
			{@const isConnected = $connectionStatus.connected && $connectionStatus.agentId === agent.id}
			<div class="cd-section p-4">
				<div class="flex items-center justify-between">
					<div class="flex items-center gap-3">
						<div class="relative">
							<Monitor class="w-6 h-6 cd-text-disabled" />
							{#if isConnected}
								<span class="cd-pulse absolute -bottom-0.5 -right-0.5"></span>
							{:else}
								<div
									class={cn(
										'absolute -bottom-0.5 -right-0.5 w-2.5 h-2.5 rounded-full border border-background',
										agent.online ? 'bg-yellow-500' : 'bg-gray-500'
									)}
								></div>
							{/if}
						</div>
						<div>
							<div class="font-medium flex items-center gap-2">
								<span class={isConnected ? 'cd-status-connected' : ''}>{agent.name || 'Unknown Agent'}</span>
								<span class="text-xs px-1.5 py-0.5 rounded bg-muted/50 border border-border/50">
									{getPlatformIcon(agent.platform)} {getPlatformLabel(agent.platform)}
								</span>
							</div>
							<div class="text-sm flex items-center gap-2">
								{#if agent.ips && agent.ips.length > 0}
									<span class="cd-mono text-xs">{agent.ips[0]}:{agent.port}</span>
								{:else}
									<span class="cd-mono text-xs">{agent.host}:{agent.port}</span>
								{/if}
								{#if agent.online}
									<Wifi class="w-3 h-3 cd-text-primary" />
								{:else}
									<WifiOff class="w-3 h-3 cd-text-disabled" />
								{/if}
							</div>
							{#if agent.version}
								<div class="text-xs cd-text-disabled">
									v{agent.version}
								</div>
							{/if}
						</div>
					</div>
					<div class="flex gap-1">
						{#if isConnected}
							<Button variant="destructive" size="icon" onclick={disconnect}>
								<LogOut class="w-4 h-4" />
							</Button>
						{:else}
							<Button
								size="icon"
								onclick={() => connect(agent.id)}
								disabled={connecting === agent.id || !agent.online}
							>
								{#if connecting === agent.id}
									<Loader2 class="w-4 h-4 animate-spin" />
								{:else}
									<LogIn class="w-4 h-4" />
								{/if}
							</Button>
						{/if}
					</div>
				</div>
			</div>
		{/each}

		{#if agents.length === 0}
			<div class="cd-section p-8 text-center space-y-4">
				<div class="cd-text-disabled">
					No agents discovered on the network.
				</div>
				<div class="text-sm cd-text-disabled">
					Make sure the CapyDeploy Agent is running on your handheld device.
				</div>
				<Button variant="gradient" onclick={refresh}>
					<RefreshCw class="w-4 h-4 mr-2" />
					Scan Network
				</Button>
			</div>
		{/if}
	</div>
</div>

<PairingDialog
	bind:open={showPairingDialog}
	agentName={pairingAgentName}
	onSuccess={handlePairingSuccess}
	onCancel={handlePairingCancel}
/>
