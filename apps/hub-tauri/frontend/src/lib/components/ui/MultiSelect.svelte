<script lang="ts">
	import { cn } from '$lib/utils';
	import { ChevronDown, Check } from 'lucide-svelte';

	interface Option {
		value: string;
		label: string;
	}

	interface Props {
		options: Option[];
		selected?: string[];
		placeholder?: string;
		disabled?: boolean;
		class?: string;
		onchange?: (selected: string[]) => void;
	}

	let {
		options,
		selected = $bindable([]),
		placeholder = 'Select...',
		disabled = false,
		class: className = '',
		onchange
	}: Props = $props();

	let open = $state(false);
	let containerRef = $state<HTMLDivElement | null>(null);

	function toggleOption(value: string) {
		if (selected.includes(value)) {
			selected = selected.filter(v => v !== value);
		} else {
			selected = [...selected, value];
		}
		onchange?.(selected);
	}

	function getDisplayText(): string {
		if (selected.length === 0) return placeholder;
		if (selected.length === options.length) return 'All';
		if (selected.length <= 2) {
			return selected.map(v => options.find(o => o.value === v)?.label || v).join(', ');
		}
		return `${selected.length} selected`;
	}

	function handleClickOutside(e: MouseEvent) {
		if (containerRef && !containerRef.contains(e.target as Node)) {
			open = false;
		}
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') open = false;
	}
</script>

<svelte:window onclick={handleClickOutside} onkeydown={handleKeydown} />

<div class="relative inline-block" bind:this={containerRef}>
	<button
		type="button"
		{disabled}
		onclick={(e) => { e.stopPropagation(); open = !open; }}
		class={cn(
			'flex h-9 w-full items-center justify-between whitespace-nowrap rounded-md border border-input bg-background px-3 py-2 text-sm shadow-sm ring-offset-background focus:outline-none focus:ring-1 focus:ring-ring disabled:cursor-not-allowed disabled:opacity-50',
			className
		)}
	>
		<span class="truncate text-left flex-1 {selected.length === 0 ? 'text-muted-foreground' : ''}">
			{getDisplayText()}
		</span>
		<ChevronDown class={cn('ml-2 h-4 w-4 opacity-50 transition-transform', open && 'rotate-180')} />
	</button>

	{#if open}
		<div class="absolute z-50 mt-1 w-full min-w-[180px] rounded-md border border-input bg-popover shadow-lg">
			<div class="max-h-60 overflow-y-auto p-1">
				{#each options as option}
					<button
						type="button"
						onclick={(e) => { e.stopPropagation(); toggleOption(option.value); }}
						class={cn(
							'flex w-full items-center gap-2 rounded-sm px-2 py-1.5 text-sm hover:bg-accent cursor-pointer',
							selected.includes(option.value) && 'bg-accent/50'
						)}
					>
						<div class={cn(
							'h-4 w-4 rounded-sm border flex items-center justify-center',
							selected.includes(option.value) ? 'bg-primary border-primary' : 'border-input'
						)}>
							{#if selected.includes(option.value)}
								<Check class="h-3 w-3 text-primary-foreground" />
							{/if}
						</div>
						<span>{option.label}</span>
					</button>
				{/each}
			</div>
		</div>
	{/if}
</div>
