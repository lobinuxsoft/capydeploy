<script lang="ts">
	import Dialog from './ui/Dialog.svelte';
	import Input from './ui/Input.svelte';
	import Button from './ui/Button.svelte';
	import { ConfirmPairing, CancelPairing } from '$lib/wailsjs';
	import { Loader2 } from 'lucide-svelte';

	interface Props {
		open?: boolean;
		agentName?: string;
		onSuccess?: () => void;
		onCancel?: () => void;
	}

	let {
		open = $bindable(false),
		agentName = 'Agent',
		onSuccess,
		onCancel
	}: Props = $props();

	let code = $state('');
	let error = $state('');
	let loading = $state(false);

	async function handleSubmit() {
		if (code.length !== 6) return;

		loading = true;
		error = '';

		try {
			await ConfirmPairing(code);
			open = false;
			code = '';
			onSuccess?.();
		} catch (e) {
			// Tauri returns error strings, not Error objects
			error = typeof e === 'string' ? e : (e instanceof Error ? e.message : 'Pairing failed');
			console.error('Pairing error:', e);
		} finally {
			loading = false;
		}
	}

	function handleCancel() {
		CancelPairing();
		open = false;
		code = '';
		error = '';
		onCancel?.();
	}

	function handleInput(e: Event) {
		const input = e.target as HTMLInputElement;
		// Only allow digits
		input.value = input.value.replace(/\D/g, '').slice(0, 6);
		code = input.value;
	}
</script>

<Dialog bind:open title="Emparejar con {agentName}" onclose={handleCancel}>
	<div class="space-y-4">
		<p class="text-sm text-muted-foreground">
			Ingresa el codigo de 6 digitos que aparece en el Agent.
		</p>

		<Input
			type="text"
			placeholder="000000"
			value={code}
			oninput={handleInput}
			class="text-center text-2xl tracking-widest font-mono"
			disabled={loading}
		/>

		{#if error}
			<p class="text-sm text-destructive">{error}</p>
		{/if}

		<div class="flex justify-end gap-2">
			<Button variant="outline" onclick={handleCancel} disabled={loading}>
				Cancel
			</Button>
			<Button
				onclick={handleSubmit}
				disabled={loading || code.length !== 6}
			>
				{#if loading}
					<Loader2 class="mr-2 h-4 w-4 animate-spin" />
				{/if}
				Emparejar
			</Button>
		</div>
	</div>
</Dialog>
