<script lang="ts">
	import { Button, Card } from '$lib/components/ui';
	import { connectionStatus } from '$lib/stores/connection';
	import type { DiscoveredAgent } from '$lib/types';
	import { Monitor, LogIn, LogOut, RefreshCw, Loader2, Wifi, WifiOff } from 'lucide-svelte';
	import { cn } from '$lib/utils';
	import {
		GetDiscoveredAgents, RefreshDiscovery, ConnectAgent, DisconnectAgent,
		GetConnectionStatus, EventsOn, EventsOff
	} from '$lib/wailsjs';
	import { browser } from '$app/environment';

	let agents = $state<DiscoveredAgent[]>([]);
	let connecting = $state<string | null>(null);
	let refreshing = $state(false);

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
		} catch (e) {
			console.error('Failed to connect:', e);
			alert('Connection failed: ' + e);
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

		// Listen for discovery events
		EventsOn('discovery:agent-found', (agent: DiscoveredAgent) => {
			agents = [...agents.filter(a => a.id !== agent.id), agent];
		});

		EventsOn('discovery:agent-updated', (agent: DiscoveredAgent) => {
			agents = agents.map(a => a.id === agent.id ? agent : a);
		});

		EventsOn('discovery:agent-lost', (agentID: string) => {
			agents = agents.filter(a => a.id !== agentID);
		});

		EventsOn('connection:changed', (status: any) => {
			connectionStatus.set(status);
		});

		// Cleanup on destroy
		return () => {
			EventsOff('discovery:agent-found');
			EventsOff('discovery:agent-updated');
			EventsOff('discovery:agent-lost');
			EventsOff('connection:changed');
		};
	});
</script>

<div class="space-y-4">
	<div class="flex items-center justify-between">
		<h3 class="text-sm font-medium text-muted-foreground">
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
			<Card class="p-4">
				<div class="flex items-center justify-between">
					<div class="flex items-center gap-3">
						<div class="relative">
							<Monitor class="w-6 h-6" />
							<div
								class={cn(
									'absolute -bottom-0.5 -right-0.5 w-2.5 h-2.5 rounded-full border border-background',
									isConnected ? 'bg-green-500' : agent.online ? 'bg-yellow-500' : 'bg-gray-500'
								)}
							></div>
						</div>
						<div>
							<div class="font-medium flex items-center gap-2">
								<span>{agent.name || 'Unknown Agent'}</span>
								<span class="text-xs px-1.5 py-0.5 rounded bg-muted">
									{getPlatformIcon(agent.platform)} {getPlatformLabel(agent.platform)}
								</span>
							</div>
							<div class="text-sm text-muted-foreground flex items-center gap-2">
								{#if agent.ips && agent.ips.length > 0}
									<span>{agent.ips[0]}:{agent.port}</span>
								{:else}
									<span>{agent.host}:{agent.port}</span>
								{/if}
								{#if agent.online}
									<Wifi class="w-3 h-3 text-green-500" />
								{:else}
									<WifiOff class="w-3 h-3 text-gray-500" />
								{/if}
							</div>
							{#if agent.version}
								<div class="text-xs text-muted-foreground">
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
			</Card>
		{/each}

		{#if agents.length === 0}
			<div class="text-center py-8 space-y-4">
				<div class="text-muted-foreground">
					No agents discovered on the network.
				</div>
				<div class="text-sm text-muted-foreground">
					Make sure the CapyDeploy Agent is running on your handheld device.
				</div>
				<Button variant="outline" onclick={refresh}>
					<RefreshCw class="w-4 h-4 mr-2" />
					Scan Network
				</Button>
			</div>
		{/if}
	</div>
</div>
