<script lang="ts">
	export interface PreviewItem {
		label: string;
		url: string;
		imgClass: string;
		placeholderClass: string;
	}

	interface Props {
		selections: PreviewItem[];
		previewUrl: (url: string) => string;
		isLocalFile: (url: string) => boolean;
	}

	let { selections, previewUrl, isLocalFile }: Props = $props();
</script>

<div class="w-56 border-l flex flex-col shrink-0">
	<div class="p-3 shrink-0">
		<h3 class="font-semibold text-sm mb-3 gradient-text">Selected Artwork</h3>
		<div class="space-y-3">
			{#each selections as item}
				<div class="flex items-center gap-3">
					<span class="w-12 text-xs text-muted-foreground shrink-0">{item.label}</span>
					{#if item.url}
						<div class="relative">
							<img
								src={previewUrl(item.url)}
								alt={item.label}
								class={item.imgClass}
							/>
							{#if isLocalFile(item.url)}
								<span class="absolute -top-1 -right-1 bg-blue-500 text-white text-[8px] px-1 rounded font-bold">LOCAL</span>
							{/if}
						</div>
					{:else}
						<div class={item.placeholderClass}>
							<span class="text-[10px] text-muted-foreground">&mdash;</span>
						</div>
					{/if}
				</div>
			{/each}
		</div>
	</div>
</div>
