<script lang="ts">
	import { Button, Card, Input } from '$lib/components/ui';
	import { connectionStatus } from '$lib/stores/connection';
	import type { InstalledGame } from '$lib/types';
	import { Folder, RefreshCw, Trash2, Loader2 } from 'lucide-svelte';
	import { GetInstalledGames, DeleteGame } from '$lib/wailsjs';
	import { cn } from '$lib/utils';

	let remotePath = $state('~/devkit-games');
	let games = $state<InstalledGame[]>([]);
	let selectedGame = $state<InstalledGame | null>(null);
	let loading = $state(false);
	let deleting = $state<string | null>(null);
	let statusMessage = $state('Connect to a device and click Refresh');

	async function refreshGames() {
		if (!$connectionStatus.connected) {
			alert('No device connected');
			return;
		}

		loading = true;
		statusMessage = 'Fetching games...';
		try {
			games = await GetInstalledGames(remotePath);
			statusMessage = `Found ${games.length} games`;
		} catch (e) {
			statusMessage = `Error: ${e}`;
			games = [];
		} finally {
			loading = false;
		}
	}

	async function deleteSelectedGame() {
		if (!selectedGame) return;

		if (!$connectionStatus.connected) {
			alert('No device connected');
			return;
		}

		if (!confirm(`Are you sure you want to delete '${selectedGame.name}'?\nThis will also remove the Steam shortcut.`)) {
			return;
		}

		const game = selectedGame;
		deleting = game.name;
		statusMessage = `Deleting ${game.name}...`;
		try {
			await DeleteGame(game.name, game.appId || 0);
			await refreshGames();
			selectedGame = null;
			statusMessage = `Deleted ${game.name}`;
		} catch (e) {
			statusMessage = `Error deleting game: ${e}`;
		} finally {
			deleting = null;
		}
	}

	function selectGame(game: InstalledGame) {
		selectedGame = game;
	}
</script>

<div class="space-y-4">
	<div class="flex items-center gap-2">
		<label class="text-sm font-medium">Games Path:</label>
		<Input bind:value={remotePath} class="flex-1" />
	</div>

	<div class="flex gap-2">
		<Button onclick={refreshGames} disabled={loading || !$connectionStatus.connected}>
			{#if loading}
				<Loader2 class="w-4 h-4 mr-2 animate-spin" />
				Loading...
			{:else}
				<RefreshCw class="w-4 h-4 mr-2" />
				Refresh
			{/if}
		</Button>
		<Button
			variant="destructive"
			onclick={deleteSelectedGame}
			disabled={!selectedGame || deleting !== null || !$connectionStatus.connected}
		>
			<Trash2 class="w-4 h-4 mr-2" />
			Delete Game
		</Button>
	</div>

	<p class="text-sm text-muted-foreground">{statusMessage}</p>

	<div class="space-y-2">
		{#each games as game}
			{@const isSelected = selectedGame?.name === game.name}
			{@const isDeleting = deleting === game.name}
			<button
				type="button"
				onclick={() => selectGame(game)}
				class={cn(
					'w-full text-left rounded-xl border bg-card text-card-foreground shadow p-4 cursor-pointer transition-all hover:bg-accent/50',
					isSelected && 'ring-2 ring-primary bg-accent'
				)}
			>
				<div class="flex items-center justify-between">
					<div class="flex items-center gap-3">
						<Folder class="w-6 h-6 text-muted-foreground" />
						<div>
							<div class="font-medium">{game.name}</div>
							<div class="text-sm text-muted-foreground">{game.path}</div>
						</div>
					</div>
					<div class="flex items-center gap-2">
						<span class="text-sm text-muted-foreground">{game.size}</span>
						{#if isDeleting}
							<Loader2 class="w-4 h-4 animate-spin" />
						{/if}
					</div>
				</div>
			</button>
		{/each}

		{#if games.length === 0 && !loading}
			<div class="text-center text-muted-foreground py-8">
				{$connectionStatus.connected
					? 'No games found. Click Refresh to scan the device.'
					: 'Connect to a device to view installed games.'}
			</div>
		{/if}
	</div>
</div>
