<script lang="ts">
	import { cn } from '$lib/utils';
	import { X } from 'lucide-svelte';
	import type { Snippet } from 'svelte';

	interface Props {
		open?: boolean;
		title?: string;
		showMascot?: boolean;
		class?: string;
		onclose?: () => void;
		children: Snippet;
	}

	let {
		open = $bindable(false),
		title = '',
		showMascot = true,
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
			class="dialog-backdrop"
			onclick={handleBackdropClick}
			aria-label="Close dialog"
		></button>

		<!-- Dialog content -->
		<div class={cn('dialog-content', className)}>
			{#if title || showMascot}
				<div class="dialog-header">
					{#if showMascot}
						<div class="dialog-mascot">
							<img src="/mascot.webp" alt="CapyDeploy" />
						</div>
					{/if}
					{#if title}
						<h2 class="dialog-title">{title}</h2>
					{/if}
					<button
						type="button"
						class="dialog-close"
						onclick={handleClose}
					>
						<X class="h-4 w-4" />
						<span class="sr-only">Close</span>
					</button>
				</div>
			{/if}
			<div class="dialog-body">
				{@render children()}
			</div>
		</div>
	</div>
{/if}

<style>
	.dialog-backdrop {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.7);
		backdrop-filter: blur(4px);
		-webkit-backdrop-filter: blur(4px);
		cursor: default;
	}

	.dialog-content {
		position: relative;
		z-index: 50;
		width: 100%;
		max-width: 28rem;
		background: linear-gradient(
			135deg,
			rgba(6, 182, 212, 0.12) 0%,
			rgba(15, 23, 42, 0.92) 40%,
			rgba(249, 115, 22, 0.06) 100%
		);
		backdrop-filter: blur(20px);
		-webkit-backdrop-filter: blur(20px);
		border: 1px solid rgba(6, 182, 212, 0.25);
		border-radius: 16px;
		box-shadow:
			0 8px 32px rgba(0, 0, 0, 0.4),
			0 0 24px rgba(6, 182, 212, 0.08),
			inset 0 1px 0 rgba(255, 255, 255, 0.06);
		overflow: hidden;
	}

	.dialog-header {
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 16px 20px;
		border-bottom: 1px solid rgba(6, 182, 212, 0.15);
		background: linear-gradient(
			90deg,
			rgba(249, 115, 22, 0.06) 0%,
			rgba(6, 182, 212, 0.08) 100%
		);
	}

	.dialog-mascot {
		width: 44px;
		height: 44px;
		flex-shrink: 0;
	}

	.dialog-mascot img {
		width: 100%;
		height: 100%;
		border-radius: 50%;
		object-fit: cover;
		-webkit-mask-image: radial-gradient(circle, #000 60%, transparent 75%);
		mask-image: radial-gradient(circle, #000 60%, transparent 75%);
	}

	.dialog-title {
		flex: 1;
		font-size: 1.15rem;
		font-weight: 600;
		background: linear-gradient(90deg, #f97316 0%, #fb923c 40%, #06b6d4 100%);
		-webkit-background-clip: text;
		-webkit-text-fill-color: transparent;
		background-clip: text;
	}

	.dialog-close {
		padding: 6px;
		border-radius: 6px;
		opacity: 0.7;
		transition: all 0.15s ease;
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid transparent;
	}

	.dialog-close:hover {
		opacity: 1;
		background: rgba(255, 255, 255, 0.1);
		border-color: rgba(255, 255, 255, 0.1);
	}

	.dialog-body {
		padding: 20px;
	}
</style>
