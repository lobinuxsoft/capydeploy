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

	function getStyles(type: Toast['type']) {
		switch (type) {
			case 'success':
				return 'border-success/50 bg-success/10';
			case 'error':
				return 'border-destructive/50 bg-destructive/10';
			case 'warning':
				return 'border-warning/50 bg-warning/10';
			case 'info':
				return 'border-primary/50 bg-primary/10';
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
			<div
				class="flex items-start gap-3 p-4 rounded-lg border backdrop-blur-sm shadow-lg animate-in slide-in-from-right-full {getStyles(t.type)}"
			>
				<Icon class="w-5 h-5 flex-shrink-0 mt-0.5 {getIconColor(t.type)}" />
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
</style>
