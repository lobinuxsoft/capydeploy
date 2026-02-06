<script lang="ts">
	import { cn } from '$lib/utils';

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
		role="switch"
		aria-checked={checked}
		{disabled}
		onclick={handleChange}
		class={cn(
			'relative inline-flex h-5 w-9 shrink-0 cursor-pointer items-center rounded-full border-2 border-transparent shadow-sm transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50',
			checked ? 'bg-primary' : 'bg-input'
		)}
	>
		<span
			class={cn(
				'pointer-events-none block h-4 w-4 rounded-full bg-background shadow-lg ring-0 transition-transform',
				checked ? 'translate-x-4' : 'translate-x-0'
			)}
		></span>
	</button>
	{#if label}
		<span class="text-sm">{label}</span>
	{/if}
</label>
