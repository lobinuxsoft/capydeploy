<script lang="ts">
	import Card from './ui/Card.svelte';
	import Button from './ui/Button.svelte';
	import { GetAuthorizedHubs, RevokeHub, EventsOn, EventsOff } from '$lib/wailsjs';
	import { Monitor, Trash2, ShieldCheck } from 'lucide-svelte';
	import { browser } from '$app/environment';

	interface AuthorizedHub {
		id: string;
		name: string;
		pairedAt: string;
		lastSeen: string;
	}

	let hubs = $state<AuthorizedHub[]>([]);
	let loading = $state(true);
	let revoking = $state<string | null>(null);

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
		if (!confirm('Revocar acceso a este Hub?')) return;

		revoking = hubId;
		try {
			await RevokeHub(hubId);
			hubs = hubs.filter(h => h.id !== hubId);
		} catch (e) {
			console.error('Failed to revoke hub:', e);
			alert('Error al revocar: ' + e);
		} finally {
			revoking = null;
		}
	}

	function formatDate(dateStr: string): string {
		try {
			const date = new Date(dateStr);
			return date.toLocaleDateString('es', {
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

<Card class="p-4">
	<div class="flex items-center gap-2 mb-4">
		<ShieldCheck class="w-5 h-5 text-primary" />
		<h3 class="font-semibold">Hubs Autorizados</h3>
	</div>

	{#if loading}
		<p class="text-sm text-muted-foreground">Cargando...</p>
	{:else if hubs.length === 0}
		<p class="text-sm text-muted-foreground">
			Ningun Hub autorizado aun. Conecta un Hub para iniciar el emparejamiento.
		</p>
	{:else}
		<div class="space-y-2">
			{#each hubs as hub}
				<div class="flex items-center justify-between p-3 rounded-lg bg-muted/50">
					<div class="flex items-center gap-3">
						<Monitor class="w-5 h-5 text-muted-foreground" />
						<div>
							<div class="font-medium">{hub.name}</div>
							<div class="text-xs text-muted-foreground">
								Emparejado: {formatDate(hub.pairedAt)}
							</div>
							<div class="text-xs text-muted-foreground">
								Ultimo uso: {formatDate(hub.lastSeen)}
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
</Card>
