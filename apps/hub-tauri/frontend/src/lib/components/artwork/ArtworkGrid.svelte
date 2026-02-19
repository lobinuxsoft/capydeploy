<script lang="ts">
	import { Button } from '$lib/components/ui';
	import type { GridData, ImageData } from '$lib/types';
	import { Check } from 'lucide-svelte';
	import { cn } from '$lib/utils';
	import { type ArtworkTabConfig, isAnimatedThumb } from './artworkTab.svelte';

	interface Props {
		config: ArtworkTabConfig;
		items: (GridData | ImageData)[];
		selectedUrl: string;
		hasMore: boolean;
		loading: boolean;
		selectedGameID: number;
		onselect: (img: GridData | ImageData) => void;
		onloadmore: () => void;
	}

	let { config, items, selectedUrl, hasMore, loading, selectedGameID, onselect, onloadmore }: Props = $props();

	const checkSize = $derived(config.compact ? 'w-2 h-2' : 'w-3 h-3');
	const badgePos = $derived(config.compact ? 'top-0.5 right-0.5' : 'top-1 right-1');
	const animBadgePos = $derived(config.compact ? 'top-0.5 left-0.5' : 'top-1 left-1');
	const animBadgeClass = $derived(config.compact
		? 'text-[7px] px-0.5'
		: 'text-[9px] px-1'
	);
</script>

<div class="text-xs text-muted-foreground mb-2">{config.description}</div>
<div class="grid gap-3" style="grid-template-columns: repeat({config.cols}, minmax(0, 1fr))">
	{#each items as img (img.url)}
		{@const isAnim = isAnimatedThumb(img.thumb)}
		{@const selected = selectedUrl === img.url}
		<button
			type="button"
			class={cn(
				'relative rounded-lg overflow-hidden border-2 transition-all',
				config.buttonClass,
				selected ? 'border-green-500 ring-2 ring-green-500/50' : 'border-transparent hover:border-blue-500'
			)}
			onclick={() => onselect(img)}
		>
			{#if isAnim}
				<video
					src={img.thumb}
					class={cn('w-full', config.aspect, `object-${config.objectFit}`, !config.buttonClass && 'bg-muted')}
					muted
					loop
					playsinline
					autoplay
				></video>
			{:else}
				<img
					src={img.thumb || img.url}
					alt=""
					class={cn('w-full', config.aspect, `object-${config.objectFit}`, !config.buttonClass && 'bg-muted')}
				/>
			{/if}
			{#if selected}
				<div class={cn('absolute bg-green-500 rounded-full p-0.5', badgePos)}>
					<Check class={cn(checkSize, 'text-white')} />
				</div>
			{/if}
			{#if isAnim}
				<span class={cn('absolute z-10 bg-orange-500 text-white rounded font-bold shadow', animBadgePos, animBadgeClass)}>ANIM</span>
			{/if}
			{#if config.dimensionStyle === 'overlay'}
				<div class="absolute bottom-0 left-0 right-0 bg-black/70 text-white text-[9px] p-0.5 text-center">
					{img.width}x{img.height}
				</div>
			{:else}
				<div class={cn('text-center text-muted-foreground', config.compact ? 'text-[8px]' : 'text-[9px]')}>
					{img.width}{#if config.showHeight}x{img.height}{/if}
				</div>
			{/if}
		</button>
	{/each}
</div>
{#if items.length === 0 && !loading && selectedGameID}
	<div class="text-center text-muted-foreground py-8 text-sm">No {config.label.toLowerCase()}s found</div>
{/if}
{#if hasMore}
	<div class="text-center py-3">
		<Button variant="outline" size="sm" onclick={onloadmore} disabled={loading}>
			Load More
		</Button>
	</div>
{/if}
