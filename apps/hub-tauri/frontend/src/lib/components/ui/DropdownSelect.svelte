<script lang="ts">
	import { cn } from '$lib/utils';
	import { ChevronDown, Check } from 'lucide-svelte';

	interface Option {
		value: string;
		label: string;
	}

	interface Props {
		options: Option[];
		value?: string;
		class?: string;
		onchange?: (value: string) => void;
	}

	let {
		options,
		value = $bindable(''),
		class: className = '',
		onchange
	}: Props = $props();

	let open = $state(false);
	let triggerEl: HTMLButtonElement | undefined = $state();
	let popoverEl: HTMLDivElement | undefined = $state();

	let selectedLabel = $derived(
		options.find((o) => o.value === value)?.label ?? options[0]?.label ?? ''
	);

	function toggle() {
		open = !open;
	}

	function select(opt: Option) {
		value = opt.value;
		open = false;
		onchange?.(opt.value);
	}

	function handleClickOutside(e: MouseEvent) {
		if (
			open &&
			triggerEl &&
			popoverEl &&
			!triggerEl.contains(e.target as Node) &&
			!popoverEl.contains(e.target as Node)
		) {
			open = false;
		}
	}

	function handleKeydown(e: KeyboardEvent) {
		if (open && e.key === 'Escape') {
			open = false;
			triggerEl?.focus();
		}
	}

	$effect(() => {
		if (open) {
			document.addEventListener('click', handleClickOutside, true);
			document.addEventListener('keydown', handleKeydown, true);
			return () => {
				document.removeEventListener('click', handleClickOutside, true);
				document.removeEventListener('keydown', handleKeydown, true);
			};
		}
	});
</script>

<div class={cn('relative inline-block', className)}>
	<button
		bind:this={triggerEl}
		type="button"
		onclick={toggle}
		class="flex items-center gap-1 text-xs bg-secondary border border-border rounded px-2 py-1.5 text-foreground hover:bg-accent transition-colors cursor-pointer"
	>
		<span>{selectedLabel}</span>
		<ChevronDown class={cn('w-3 h-3 transition-transform', open && 'rotate-180')} />
	</button>

	{#if open}
		<div
			bind:this={popoverEl}
			class="absolute top-full left-0 mt-1 z-50 min-w-[140px] bg-popover border border-border rounded shadow-lg py-1"
		>
			{#each options as opt (opt.value)}
				<button
					type="button"
					onclick={() => select(opt)}
					class={cn(
						'w-full flex items-center gap-2 px-3 py-1.5 text-xs text-left hover:bg-accent transition-colors cursor-pointer',
						opt.value === value ? 'text-foreground' : 'text-muted-foreground'
					)}
				>
					{#if opt.value === value}
						<Check class="w-3 h-3 shrink-0" />
					{:else}
						<span class="w-3 shrink-0"></span>
					{/if}
					{opt.label}
				</button>
			{/each}
		</div>
	{/if}
</div>
