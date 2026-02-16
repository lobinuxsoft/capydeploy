<script lang="ts">
	import { cn } from '$lib/utils';
	import { Check } from 'lucide-svelte';

	interface Props {
		checked?: boolean;
		disabled?: boolean;
		label?: string;
		class?: string;
		onchange?: (checked: boolean) => void;
	}

	let {
		checked = $bindable(false),
		disabled = false,
		label = '',
		class: className = '',
		onchange
	}: Props = $props();

	function handleChange() {
		if (!disabled) {
			checked = !checked;
			onchange?.(checked);
		}
	}
</script>

<label class={cn('flex items-center gap-2 cursor-pointer', disabled && 'cursor-not-allowed opacity-50', className)}>
	<button
		type="button"
		role="checkbox"
		aria-checked={checked}
		{disabled}
		onclick={handleChange}
		class={cn(
			'peer h-4 w-4 shrink-0 rounded-sm border border-primary shadow focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50',
			checked && 'bg-primary text-primary-foreground'
		)}
	>
		{#if checked}
			<Check class="h-3 w-3 m-auto" />
		{/if}
	</button>
	{#if label}
		<span class="text-sm">{label}</span>
	{/if}
</label>
