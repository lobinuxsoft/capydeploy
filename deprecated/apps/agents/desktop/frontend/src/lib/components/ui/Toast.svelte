<script lang="ts">
	import { toast, type Toast } from '$lib/stores/toast';
	import { X, CheckCircle, AlertCircle, AlertTriangle, Info } from 'lucide-svelte';

	let toasts = $state<Toast[]>([]);

	$effect(() => {
		const unsubscribe = toast.subscribe((value) => {
			toasts = value;
		});
		return unsubscribe;
	});

	function getIcon(type: Toast['type']) {
		switch (type) {
			case 'success':
				return CheckCircle;
			case 'error':
				return AlertCircle;
			case 'warning':
				return AlertTriangle;
			case 'info':
				return Info;
		}
	}

	function getIconColor(type: Toast['type']) {
		switch (type) {
			case 'success':
				return 'text-success';
			case 'error':
				return 'text-destructive';
			case 'warning':
				return 'text-warning';
			case 'info':
				return 'text-primary';
		}
	}
</script>

{#if toasts.length > 0}
	<div class="fixed bottom-4 right-4 z-50 flex flex-col gap-2 max-w-sm">
		{#each toasts as t (t.id)}
			{@const Icon = getIcon(t.type)}
			<div class="toast-container animate-in slide-in-from-right-full">
				<div class="toast-mascot">
					<img src="/mascot.gif" alt="CapyDeploy" />
				</div>
				<div class="toast-content">
					<Icon class="w-5 h-5 flex-shrink-0 {getIconColor(t.type)}" />
					<div class="flex-1 min-w-0">
						<p class="font-medium text-sm">{t.title}</p>
						{#if t.message}
							<p class="text-xs text-muted-foreground mt-1">{t.message}</p>
						{/if}
					</div>
					<button
						type="button"
						onclick={() => toast.remove(t.id)}
						class="flex-shrink-0 p-1 rounded hover:bg-secondary/50 transition-colors"
					>
						<X class="w-4 h-4 text-muted-foreground" />
					</button>
				</div>
			</div>
		{/each}
	</div>
{/if}

<style>
	@keyframes slide-in-from-right-full {
		from {
			transform: translateX(100%);
			opacity: 0;
		}
		to {
			transform: translateX(0);
			opacity: 1;
		}
	}

	.animate-in {
		animation: slide-in-from-right-full 0.3s ease-out;
	}

	.toast-container {
		display: flex;
		align-items: stretch;
		background: linear-gradient(
			135deg,
			rgba(6, 182, 212, 0.12) 0%,
			rgba(15, 23, 42, 0.85) 40%,
			rgba(249, 115, 22, 0.06) 100%
		);
		backdrop-filter: blur(16px);
		-webkit-backdrop-filter: blur(16px);
		border: 1px solid rgba(6, 182, 212, 0.25);
		border-radius: 12px;
		overflow: hidden;
		box-shadow:
			0 4px 16px rgba(0, 0, 0, 0.3),
			0 0 12px rgba(6, 182, 212, 0.08),
			inset 0 1px 0 rgba(255, 255, 255, 0.05);
	}

	.toast-mascot {
		width: 48px;
		display: flex;
		align-items: center;
		justify-content: center;
		background: linear-gradient(180deg, rgba(6, 182, 212, 0.15) 0%, rgba(249, 115, 22, 0.1) 100%);
		border-right: 1px solid rgba(6, 182, 212, 0.15);
		flex-shrink: 0;
	}

	.toast-mascot img {
		width: 32px;
		height: 32px;
		border-radius: 50%;
		object-fit: cover;
		-webkit-mask-image: radial-gradient(circle, #000 60%, transparent 75%);
		mask-image: radial-gradient(circle, #000 60%, transparent 75%);
	}

	.toast-content {
		display: flex;
		align-items: flex-start;
		gap: 12px;
		padding: 12px 14px;
		flex: 1;
	}
</style>
