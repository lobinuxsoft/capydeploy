<script lang="ts">
	import { cn } from '$lib/utils';
	import type { Snippet } from 'svelte';

	interface Props {
		class?: string;
		children: Snippet;
	}

	let { class: className = '', children }: Props = $props();
</script>

<div class={cn('rounded-xl border bg-card text-card-foreground shadow-lg glass-card card-hover backdrop-blur-sm', className)}>
	{@render children()}
</div>
