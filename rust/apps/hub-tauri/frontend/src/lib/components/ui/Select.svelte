<script lang="ts">
	import { cn } from '$lib/utils';
	import { ChevronDown } from 'lucide-svelte';

	interface Props {
		options: string[];
		value?: string;
		placeholder?: string;
		disabled?: boolean;
		class?: string;
		onchange?: (value: string) => void;
	}

	let {
		options,
		value = '',
		placeholder = 'Select...',
		disabled = false,
		class: className = '',
		onchange
	}: Props = $props();

	function handleChange(e: Event) {
		const target = e.target as HTMLSelectElement;
		onchange?.(target.value);
	}
</script>

<div class="relative inline-block">
	<select
		{disabled}
		onchange={handleChange}
		class={cn(
			'flex h-9 w-full items-center justify-between whitespace-nowrap rounded-md border border-input bg-background px-3 py-2 pr-8 text-sm shadow-sm ring-offset-background focus:outline-none focus:ring-1 focus:ring-ring disabled:cursor-not-allowed disabled:opacity-50 appearance-none cursor-pointer',
			className
		)}
	>
		{#if placeholder}
			<option value="" disabled selected={!value}>{placeholder}</option>
		{/if}
		{#each options as option}
			<option value={option} selected={value === option}>{option}</option>
		{/each}
	</select>
	<ChevronDown class="absolute right-2 top-1/2 -translate-y-1/2 h-4 w-4 opacity-50 pointer-events-none" />
</div>
