<script lang="ts">
	import { Button, Card } from '$lib/components/ui';
	import ArtworkSelector from '$lib/components/ArtworkSelector.svelte';
	import { connectionStatus } from '$lib/stores/connection';
	import { toast } from '$lib/stores/toast';
	import type { InstalledGame, ArtworkSelection } from '$lib/types';
	import { Folder, RefreshCw, Trash2, Pencil, Loader2 } from 'lucide-svelte';
	import { GetInstalledGames, DeleteGame, GetAgentInstallPath, UpdateGameArtwork } from '$lib/wailsjs';

	let installPath = $state('');
	let games = $state<InstalledGame[]>([]);
	let loading = $state(false);
	let deleting = $state<string | null>(null);
	let editingGame = $state<InstalledGame | null>(null);
	let showArtworkSelector = $state(false);
	let savingArtwork = $state(false);
	let statusMessage = $state('Connect to a device and click Refresh');

	async function refreshGames() {
		if (!$connectionStatus.connected) {
			toast.warning('No connection', 'Connect to a device first');
			return;
		}

		loading = true;
		statusMessage = 'Searching for games...';
		try {
			// Get install path from agent
			installPath = await GetAgentInstallPath();
			games = await GetInstalledGames('');
			statusMessage = `${games.length} games found`;
		} catch (e) {
			statusMessage = `Error: ${e}`;
			toast.error('Error', String(e));
			games = [];
		} finally {
			loading = false;
		}
	}

	async function deleteGame(game: InstalledGame) {
		if (!$connectionStatus.connected) {
			toast.warning('No connection', 'Connect to a device first');
			return;
		}

		deleting = game.name;
		statusMessage = `Deleting ${game.name}...`;
		try {
			await DeleteGame(game.name, game.appId || 0);
			await refreshGames();
			toast.success('Game deleted', game.name);
		} catch (e) {
			toast.error('Error deleting', String(e));
			statusMessage = `Error: ${e}`;
		} finally {
			deleting = null;
		}
	}

	function editArtwork(game: InstalledGame) {
		if (!$connectionStatus.connected) {
			toast.warning('No connection', 'Connect to a device first');
			return;
		}
		editingGame = game;
		showArtworkSelector = true;
	}

	async function handleArtworkSave(selection: ArtworkSelection) {
		if (!editingGame) return;

		savingArtwork = true;
		statusMessage = `Updating artwork for ${editingGame.name}...`;
		try {
			await UpdateGameArtwork(
				editingGame.appId || 0,
				selection.gridPortrait,
				selection.heroImage,
				selection.logoImage,
				selection.iconImage,
				selection.gridDBGameID
			);
			toast.success('Artwork updated', editingGame.name);
			statusMessage = `Artwork updated for ${editingGame.name}`;
		} catch (e) {
			toast.error('Error updating artwork', String(e));
			statusMessage = `Error: ${e}`;
		} finally {
			savingArtwork = false;
			showArtworkSelector = false;
			editingGame = null;
		}
	}

	function handleArtworkClose() {
		showArtworkSelector = false;
		editingGame = null;
	}
</script>

<div class="space-y-4">
	{#if installPath}
		<p class="text-sm cd-text-disabled">
			Install path: <span class="cd-mono">{installPath}</span>
		</p>
	{/if}

	<div class="flex gap-2">
		<Button variant="gradient" onclick={refreshGames} disabled={loading || !$connectionStatus.connected}>
			{#if loading}
				<Loader2 class="w-4 h-4 mr-2 animate-spin" />
				Loading...
			{:else}
				<RefreshCw class="w-4 h-4 mr-2" />
				Refresh
			{/if}
		</Button>
	</div>

	<p class="text-sm cd-text-disabled">{statusMessage}</p>

	<div class="space-y-2">
		{#each games as game}
			{@const isDeleting = deleting === game.name}
			<div class="cd-section p-4">
				<div class="flex items-center justify-between">
					<div class="flex items-center gap-3">
						<Folder class="w-6 h-6 cd-text-disabled" />
						<div>
							<div class="font-medium cd-value">{game.name}</div>
							<div class="text-sm cd-text-disabled">{game.path}</div>
						</div>
					</div>
					<div class="flex items-center gap-3">
						{#if game.size && game.size !== 'N/A'}
							<span class="text-sm cd-mono">{game.size}</span>
						{/if}
						<Button
							variant="ghost"
							size="icon"
							onclick={() => editArtwork(game)}
							disabled={!$connectionStatus.connected || savingArtwork}
							class="hover:bg-accent"
						>
							{#if savingArtwork && editingGame?.name === game.name}
								<Loader2 class="w-4 h-4 animate-spin" />
							{:else}
								<Pencil class="w-4 h-4" />
							{/if}
						</Button>
						<Button
							variant="ghost"
							size="icon"
							onclick={() => deleteGame(game)}
							disabled={isDeleting || !$connectionStatus.connected}
							class="text-destructive hover:text-destructive hover:bg-destructive/10"
						>
							{#if isDeleting}
								<Loader2 class="w-4 h-4 animate-spin" />
							{:else}
								<Trash2 class="w-4 h-4" />
							{/if}
						</Button>
					</div>
				</div>
			</div>
		{/each}

		{#if games.length === 0 && !loading}
			<div class="cd-section p-8 text-center cd-text-disabled">
				{$connectionStatus.connected
					? 'No games found. Click Refresh to scan the device.'
					: 'Connect to a device to view installed games.'}
			</div>
		{/if}
	</div>
</div>

{#if showArtworkSelector && editingGame}
	<ArtworkSelector
		gameName={editingGame.name}
		currentSelection={null}
		onsave={handleArtworkSave}
		onclose={handleArtworkClose}
	/>
{/if}
