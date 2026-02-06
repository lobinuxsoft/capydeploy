<script lang="ts">
	import Card from './ui/Card.svelte';
	import Button from './ui/Button.svelte';
	import { GetAuthorizedHubs, RevokeHub, EventsOn, EventsOff } from '$lib/wailsjs';
	import { toast } from '$lib/stores/toast';
	import { Monitor, Trash2, ShieldCheck, ChevronDown, ChevronRight } from 'lucide-svelte';
	import { browser } from '$app/environment';

	interface AuthorizedHub {
		id: string;
		name: string;
		platform?: string;
		pairedAt: string;
		lastSeen: string;
	}

	function getPlatformIcon(platform?: string): string {
		switch (platform) {
			case 'windows': return '🪟';
			case 'darwin': return '🍎';
			case 'linux': return '🐧';
			default: return '💻';
		}
	}

	function getPlatformName(platform?: string): string {
		switch (platform) {
			case 'windows': return 'Windows';
			case 'darwin': return 'macOS';
			case 'linux': return 'Linux';
			default: return platform || 'Unknown';
		}
	}

	let hubs = $state<AuthorizedHub[]>([]);
	let loading = $state(true);
	let revoking = $state<string | null>(null);
	let expanded = $state(true);

	async function loadHubs() {
		try {
			const list = await GetAuthorizedHubs();
			hubs = list || [];
		} catch (e) {
			console.error('Failed to load authorized hubs:', e);
		} finally {
			loading = false;
		}
	}

	async function handleRevoke(hubId: string) {
		revoking = hubId;
		try {
			await RevokeHub(hubId);
			hubs = hubs.filter(h => h.id !== hubId);
			toast.success('Hub revoked');
		} catch (e) {
			console.error('Failed to revoke hub:', e);
			toast.error('Error revoking', String(e));
		} finally {
			revoking = null;
		}
	}

	function formatDate(dateStr: string): string {
		try {
			const date = new Date(dateStr);
			return date.toLocaleDateString('en', {
				year: 'numeric',
				month: 'short',
				day: 'numeric',
				hour: '2-digit',
				minute: '2-digit'
			});
		} catch {
			return dateStr;
		}
	}

	$effect(() => {
		if (!browser) return;

		loadHubs();

		// Listen for hub changes (new pairing or revocation)
		EventsOn('hubs:changed', () => {
			loadHubs();
		});

		// Listen for hub revocation events
		EventsOn('auth:hub-revoked', (hubId: string) => {
			hubs = hubs.filter(h => h.id !== hubId);
		});

		return () => {
			EventsOff('hubs:changed');
			EventsOff('auth:hub-revoked');
		};
	});
</script>

<div class="cd-section p-4">
	<button
		type="button"
		class="w-full flex items-center gap-2 hover:text-primary transition-colors"
		onclick={() => expanded = !expanded}
	>
		{#if expanded}
			<ChevronDown class="w-4 h-4 cd-text-primary" />
		{:else}
			<ChevronRight class="w-4 h-4 cd-text-disabled" />
		{/if}
		<ShieldCheck class="w-5 h-5 cd-text-primary" />
		<h3 class="cd-section-title">Authorized Hubs</h3>
	</button>

	{#if expanded}
		<div class="mt-4">
			{#if loading}
				<p class="text-sm cd-text-disabled">Loading...</p>
			{:else if hubs.length === 0}
				<p class="text-sm cd-text-disabled">
					No authorized Hubs yet. Connect a Hub to start pairing.
				</p>
			{:else}
				<div class="space-y-2">
					{#each hubs as hub}
						<div class="flex items-center justify-between p-3 rounded-lg bg-secondary/30 border border-border/50">
							<div class="flex items-center gap-3">
								<Monitor class="w-5 h-5 cd-text-disabled" />
								<div>
									<div class="cd-value font-medium flex items-center gap-2">
										{hub.name}
										{#if hub.platform}
											<span class="text-xs cd-text-disabled" title={getPlatformName(hub.platform)}>
												{getPlatformIcon(hub.platform)}
											</span>
										{/if}
									</div>
									<div class="text-xs cd-text-capy">
										Paired: {formatDate(hub.pairedAt)}
									</div>
									<div class="text-xs cd-text-disabled">
										Last seen: {formatDate(hub.lastSeen)}
									</div>
								</div>
							</div>
							<Button
								variant="ghost"
								size="icon"
								onclick={() => handleRevoke(hub.id)}
								disabled={revoking === hub.id}
							>
								<Trash2 class="w-4 h-4 text-destructive" />
							</Button>
						</div>
					{/each}
				</div>
			{/if}
		</div>
	{/if}
</div>
