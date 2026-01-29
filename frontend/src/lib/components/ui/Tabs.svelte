<script lang="ts">
	import { cn } from '$lib/utils';
	import type { Snippet } from 'svelte';

	interface Tab {
		id: string;
		label: string;
	}

	interface Props {
		tabs: Tab[];
		activeTab?: string;
		class?: string;
		children: Snippet<[string]>;
	}

	let {
		tabs,
		activeTab = $bindable(tabs[0]?.id ?? ''),
		class: className = '',
		children
	}: Props = $props();
</script>

<div class={cn('w-full', className)}>
	<!-- Tab list -->
	<div class="inline-flex h-9 items-center justify-center rounded-lg bg-muted p-1 text-muted-foreground mb-4">
		{#each tabs as tab}
			<button
				type="button"
				onclick={() => activeTab = tab.id}
				class={cn(
					'inline-flex items-center justify-center whitespace-nowrap rounded-md px-3 py-1 text-sm font-medium ring-offset-background transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50',
					activeTab === tab.id
						? 'bg-background text-foreground shadow'
						: 'hover:bg-background/50'
				)}
			>
				{tab.label}
			</button>
		{/each}
	</div>

	<!-- Tab content -->
	<div class="mt-2">
		{@render children(activeTab)}
	</div>
</div>
