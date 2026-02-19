<script lang="ts">
	import { Button, Input } from '$lib/components/ui';
	import type { SearchResult } from '$lib/types';
	import { Search, Loader2 } from 'lucide-svelte';
	import { cn } from '$lib/utils';

	interface Props {
		searchQuery: string;
		searching: boolean;
		searchResults: SearchResult[];
		selectedGameID: number;
		selectedGameName: string;
		onsearch: () => void;
		onselectgame: (game: SearchResult) => void;
	}

	let {
		searchQuery = $bindable(),
		searching,
		searchResults,
		selectedGameID,
		selectedGameName,
		onsearch,
		onselectgame
	}: Props = $props();
</script>

<div class="w-56 border-r flex flex-col shrink-0">
	<div class="p-3 space-y-2 shrink-0">
		<h3 class="font-semibold text-sm gradient-text">Search SteamGridDB</h3>
		<div class="flex gap-1">
			<Input
				bind:value={searchQuery}
				placeholder="Game name..."
				class="text-sm"
				onkeydown={(e) => e.key === 'Enter' && onsearch()}
			/>
			<Button size="icon" onclick={onsearch} disabled={searching}>
				{#if searching}
					<Loader2 class="w-4 h-4 animate-spin" />
				{:else}
					<Search class="w-4 h-4" />
				{/if}
			</Button>
		</div>
		{#if selectedGameName}
			<p class="text-xs text-green-500 truncate">
				{selectedGameName}
			</p>
		{/if}
	</div>
	<div class="flex-1 overflow-y-auto min-h-0">
		{#each searchResults as game}
			<button
				type="button"
				class={cn(
					'w-full p-2 text-left hover:bg-accent border-b text-xs',
					selectedGameID === game.id && 'bg-accent'
				)}
				onclick={() => onselectgame(game)}
			>
				<div class="font-medium truncate">{game.name}</div>
				{#if game.verified}
					<span class="text-[10px] text-green-500">[Verified]</span>
				{/if}
			</button>
		{/each}
	</div>
</div>
