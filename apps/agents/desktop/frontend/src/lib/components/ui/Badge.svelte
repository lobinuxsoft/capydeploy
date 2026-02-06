<script lang="ts">
	import { cn } from '$lib/utils';
	import type { Snippet } from 'svelte';

	interface Props {
		variant?: 'default' | 'secondary' | 'destructive' | 'outline' | 'success' | 'warning';
		class?: string;
		children: Snippet;
	}

	let { variant = 'default', class: className = '', children }: Props = $props();

	const variants = {
		default: 'border-transparent bg-primary text-primary-foreground shadow hover:bg-primary/80',
		secondary: 'border-transparent bg-secondary text-secondary-foreground hover:bg-secondary/80',
		destructive: 'border-transparent bg-destructive text-destructive-foreground shadow hover:bg-destructive/80',
		outline: 'text-foreground',
		success: 'border-transparent bg-success text-primary-foreground shadow',
		warning: 'border-transparent bg-warning text-primary-foreground shadow'
	};
</script>

<span class={cn(
	'inline-flex items-center rounded-md border px-2.5 py-0.5 text-xs font-semibold transition-colors focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2',
	variants[variant],
	className
)}>
	{@render children()}
</span>
