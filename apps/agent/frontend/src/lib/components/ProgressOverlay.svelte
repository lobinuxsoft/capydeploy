<script lang="ts">
	import { browser } from '$app/environment';
	import { EventsOn, EventsOff } from '$lib/wailsjs';
	import { Download, Trash2 } from 'lucide-svelte';

	interface OperationEvent {
		type: 'install' | 'delete';
		status: 'start' | 'progress' | 'complete' | 'error';
		gameName: string;
		progress: number;
		message?: string;
	}

	let visible = $state(false);
	let operation = $state<OperationEvent | null>(null);
	let hideTimeout: ReturnType<typeof setTimeout> | null = null;

	function handleOperation(event: OperationEvent) {
		// Clear any pending hide timeout
		if (hideTimeout) {
			clearTimeout(hideTimeout);
			hideTimeout = null;
		}

		operation = event;

		if (event.status === 'start' || event.status === 'progress') {
			visible = true;
		} else if (event.status === 'complete' || event.status === 'error') {
			// Keep visible for a moment then hide
			hideTimeout = setTimeout(() => {
				visible = false;
				operation = null;
			}, 1500);
		}
	}

	$effect(() => {
		if (!browser) return;

		EventsOn('operation', handleOperation);

		return () => {
			EventsOff('operation');
			if (hideTimeout) {
				clearTimeout(hideTimeout);
			}
		};
	});

	function getOperationLabel(type: string): string {
		switch (type) {
			case 'install':
				return 'Instalando';
			case 'delete':
				return 'Eliminando';
			default:
				return 'Procesando';
		}
	}

	function getStatusColor(status: string): string {
		switch (status) {
			case 'complete':
				return 'bg-success';
			case 'error':
				return 'bg-destructive';
			default:
				return 'bg-primary';
		}
	}
</script>

{#if visible && operation}
	<div class="fixed inset-0 z-50 flex items-center justify-center bg-background/80 backdrop-blur-sm">
		<div class="w-full max-w-md mx-4 p-6 rounded-lg border bg-card shadow-lg">
			<div class="flex items-center gap-4 mb-4">
				<div class="p-3 rounded-full bg-primary/10">
					{#if operation.type === 'install'}
						<Download class="w-6 h-6 text-primary" />
					{:else}
						<Trash2 class="w-6 h-6 text-destructive" />
					{/if}
				</div>
				<div class="flex-1 min-w-0">
					<h3 class="font-semibold">
						{getOperationLabel(operation.type)}
					</h3>
					<p class="text-sm text-muted-foreground truncate">
						{operation.gameName || 'Juego'}
					</p>
				</div>
			</div>

			<!-- Progress bar -->
			<div class="space-y-2">
				<div class="h-2 rounded-full bg-secondary overflow-hidden">
					<div
						class="h-full transition-all duration-300 {getStatusColor(operation.status)}"
						style="width: {operation.progress}%"
					></div>
				</div>
				<div class="flex justify-between text-sm">
					<span class="text-muted-foreground">
						{#if operation.message}
							{operation.message}
						{:else if operation.status === 'progress'}
							Transfiriendo archivos...
						{:else if operation.status === 'complete'}
							Completado
						{:else if operation.status === 'error'}
							Error
						{:else}
							Iniciando...
						{/if}
					</span>
					<span class="font-mono">{operation.progress.toFixed(0)}%</span>
				</div>
			</div>
		</div>
	</div>
{/if}
