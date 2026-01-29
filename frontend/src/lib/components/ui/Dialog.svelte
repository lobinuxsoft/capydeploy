<script lang="ts">
	import { cn } from '$lib/utils';
	import { X } from 'lucide-svelte';
	import type { Snippet } from 'svelte';

	interface Props {
		open?: boolean;
		title?: string;
		class?: string;
		onclose?: () => void;
		children: Snippet;
	}

	let {
		open = $bindable(false),
		title = '',
		class: className = '',
		onclose,
		children
	}: Props = $props();

	function handleClose() {
		open = false;
		onclose?.();
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') handleClose();
	}

	function handleBackdropClick(e: MouseEvent) {
		if (e.target === e.currentTarget) handleClose();
	}
</script>

<svelte:window onkeydown={handleKeydown} />

{#if open}
	<div
		class="fixed inset-0 z-50 flex items-center justify-center"
		role="dialog"
		aria-modal="true"
	>
		<!-- Backdrop -->
		<button
			type="button"
			class="fixed inset-0 bg-black/80 cursor-default"
			onclick={handleBackdropClick}
			aria-label="Close dialog"
		></button>

		<!-- Dialog content -->
		<div class={cn(
			'relative z-50 w-full max-w-lg rounded-lg border bg-background p-6 shadow-lg',
			className
		)}>
			{#if title}
				<div class="flex items-center justify-between mb-4">
					<h2 class="text-lg font-semibold">{title}</h2>
					<button
						type="button"
						class="rounded-sm opacity-70 ring-offset-background transition-opacity hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2"
						onclick={handleClose}
					>
						<X class="h-4 w-4" />
						<span class="sr-only">Close</span>
					</button>
				</div>
			{/if}
			{@render children()}
		</div>
	</div>
{/if}
