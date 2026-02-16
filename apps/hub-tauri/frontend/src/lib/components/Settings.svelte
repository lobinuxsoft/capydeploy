<script lang="ts">
	import { Button, Card, Input } from '$lib/components/ui';
	import { toast } from '$lib/stores/toast';
	import { ExternalLink, Save, Loader2, Info, Server, RotateCcw, FolderOpen } from 'lucide-svelte';
	import {
		GetSteamGridDBAPIKey, SetSteamGridDBAPIKey,
		GetVersion,
		GetHubInfo, SetHubName,
		GetGameLogDirectory, SetGameLogDirectory, SelectFolder
	} from '$lib/wailsjs';
	import { open } from '@tauri-apps/plugin-shell';
	import { browser } from '$app/environment';
	import type { VersionInfo } from '$lib/types';
	import { consoleColors, DEFAULT_COLORS, type ConsoleColors } from '$lib/stores/consolelog';

	let hubName = $state('');
	let hubId = $state('');
	let hubPlatform = $state('');
	let savingHubName = $state(false);
	let apiKey = $state('');
	let saving = $state(false);
	let versionInfo = $state<VersionInfo | null>(null);

	let logColors = $state<ConsoleColors>({ ...DEFAULT_COLORS });
	let gameLogDir = $state('');
	let savingGameLogDir = $state(false);

	const colorLabels: { key: keyof ConsoleColors; label: string }[] = [
		{ key: 'error', label: 'Error' },
		{ key: 'warn', label: 'Warning' },
		{ key: 'info', label: 'Info' },
		{ key: 'debug', label: 'Debug' },
		{ key: 'log', label: 'Log' }
	];

	async function loadSettings() {
		if (!browser) return;

		try {
			const info = await GetHubInfo();
			hubName = info.name || '';
			hubId = info.id || '';
			hubPlatform = info.platform || '';
		} catch (e) {
			console.error('Failed to load hub info:', e);
		}

		try {
			const key = await GetSteamGridDBAPIKey();
			apiKey = key || '';
		} catch (e) {
			console.error('Failed to load API key:', e);
		}

		try {
			versionInfo = await GetVersion();
		} catch (e) {
			console.error('Failed to load version:', e);
		}

		try {
			gameLogDir = (await GetGameLogDirectory()) || '';
		} catch (e) {
			console.error('Failed to load game log directory:', e);
		}
	}

	async function saveSettings() {
		saving = true;
		try {
			await SetSteamGridDBAPIKey(apiKey);
			toast.success('Settings saved');
		} catch (e) {
			toast.error('Error saving settings', String(e));
		} finally {
			saving = false;
		}
	}

	async function saveHubName() {
		savingHubName = true;
		try {
			await SetHubName(hubName);
			toast.success('Hub name updated');
		} catch (e) {
			toast.error('Error saving', String(e));
		} finally {
			savingHubName = false;
		}
	}

	async function selectGameLogDir() {
		try {
			const dir = await SelectFolder();
			if (dir) {
				gameLogDir = dir;
				await saveGameLogDir();
			}
		} catch (e) {
			// User cancelled
		}
	}

	async function saveGameLogDir() {
		savingGameLogDir = true;
		try {
			await SetGameLogDirectory(gameLogDir);
			toast.success('Game log directory saved');
		} catch (e) {
			toast.error('Error', String(e));
		} finally {
			savingGameLogDir = false;
		}
	}

	async function clearGameLogDir() {
		gameLogDir = '';
		await saveGameLogDir();
	}

	function openSteamGridDBApiPage() {
		open('https://www.steamgriddb.com/profile/preferences/api');
	}

	$effect(() => {
		if (!browser) return;
		loadSettings();
		const unsub = consoleColors.subscribe((c) => (logColors = c));
		return unsub;
	});

	function handleColorChange(key: keyof ConsoleColors, value: string) {
		consoleColors.updateColors({ [key]: value });
	}

	function resetLogColors() {
		consoleColors.resetColors();
	}
</script>

<div class="space-y-4">
	<!-- Hub Identity Section -->
	<div class="cd-section p-4">
		<h3 class="cd-section-title">Hub Identity</h3>
		<p class="text-sm cd-text-disabled mb-4">
			This name will be shown to agents when they connect.
		</p>

		<div class="space-y-4">
			<div class="space-y-2">
				<label class="text-sm font-medium">Hub Name</label>
				<div class="flex gap-2">
					<Input
						type="text"
						bind:value={hubName}
						placeholder="My Gaming PC"
						class="flex-1"
					/>
					<Button onclick={saveHubName} disabled={savingHubName} variant="outline">
						{#if savingHubName}
							<Loader2 class="w-4 h-4 animate-spin" />
						{:else}
							<Save class="w-4 h-4" />
						{/if}
					</Button>
				</div>
			</div>

			<div class="space-y-2 text-sm pl-2">
				<div class="flex items-center gap-2">
					<Server class="w-4 h-4 cd-text-disabled" />
					<span class="cd-text-disabled">Hub ID:</span>
					<span class="cd-mono text-xs">{hubId || 'Loading...'}</span>
				</div>
				<div class="flex items-center gap-2">
					<span class="cd-text-disabled ml-6">Platform:</span>
					<span class="cd-value">{hubPlatform || 'Loading...'}</span>
				</div>
			</div>
		</div>
	</div>

	<!-- SteamGridDB Section -->
	<div class="cd-section p-4">
		<h3 class="cd-section-title">SteamGridDB Integration</h3>
		<p class="text-sm cd-text-disabled mb-4">
			SteamGridDB allows you to select custom artwork for your games.
		</p>
		<p class="text-sm mb-4">
			Get your API key from
			<button
				onclick={openSteamGridDBApiPage}
				class="cd-text-primary hover:underline inline-flex items-center gap-1"
			>
				steamgriddb.com/profile/preferences/api
				<ExternalLink class="w-3 h-3" />
			</button>
		</p>

		<div class="space-y-2">
			<label class="text-sm font-medium">API Key</label>
			<Input
				type="password"
				bind:value={apiKey}
				placeholder="Your SteamGridDB API key"
			/>
		</div>
	</div>

	<Button variant="gradient" onclick={saveSettings} disabled={saving} class="w-full">
		{#if saving}
			<Loader2 class="w-4 h-4 mr-2 animate-spin" />
		{:else}
			<Save class="w-4 h-4 mr-2" />
		{/if}
		Save Settings
	</Button>

	<!-- Game Log Directory -->
	<div class="cd-section p-4">
		<h3 class="cd-section-title">Game Log Directory</h3>
		<p class="text-sm cd-text-disabled mb-4">
			Save console and game log entries to text files on disk. Leave empty to disable.
		</p>

		<div class="space-y-2">
			<div class="flex gap-2">
				<Input
					type="text"
					bind:value={gameLogDir}
					placeholder="Not configured (file logging disabled)"
					class="flex-1"
					readonly
				/>
				<Button onclick={selectGameLogDir} disabled={savingGameLogDir} variant="outline">
					<FolderOpen class="w-4 h-4" />
				</Button>
				{#if gameLogDir}
					<Button onclick={clearGameLogDir} disabled={savingGameLogDir} variant="outline" class="text-destructive">
						Clear
					</Button>
				{/if}
			</div>
			{#if gameLogDir}
				<p class="text-xs cd-text-disabled">
					Logs will be saved to: <span class="cd-mono">{gameLogDir}</span>
				</p>
			{/if}
		</div>
	</div>

	<!-- Console Log Colors -->
	<div class="cd-section p-4">
		<div class="flex items-center justify-between mb-4">
			<div>
				<h3 class="cd-section-title">Console Log Colors</h3>
				<p class="text-sm cd-text-disabled">Customize log level colors in the console viewer.</p>
			</div>
			<Button variant="outline" onclick={resetLogColors} class="text-xs">
				<RotateCcw class="w-3 h-3 mr-1" />
				Reset
			</Button>
		</div>

		<div class="grid grid-cols-2 sm:grid-cols-3 gap-3">
			{#each colorLabels as { key, label }}
				<div class="flex items-center gap-2">
					<input
						type="color"
						value={logColors[key]}
						oninput={(e) => handleColorChange(key, (e.target as HTMLInputElement).value)}
						class="w-8 h-8 rounded border border-border cursor-pointer bg-transparent"
					/>
					<div class="flex flex-col">
						<span class="text-sm font-medium">{label}</span>
						<span class="text-[10px] font-mono cd-text-disabled">{logColors[key]}</span>
					</div>
					<span
						class="ml-auto text-xs font-mono px-2 py-0.5 rounded"
						style="color: {logColors[key]}; background: {logColors[key]}20"
					>
						sample
					</span>
				</div>
			{/each}
		</div>
	</div>

	<!-- About Section -->
	<div class="cd-section p-4">
		<h3 class="cd-section-title">About</h3>
		<div class="flex items-center gap-2 mb-3">
			<Info class="w-5 h-5 cd-text-disabled" />
			<span class="font-medium cd-value">CapyDeploy Hub</span>
		</div>
		{#if versionInfo}
			<div class="space-y-2 text-sm">
				<div class="flex justify-between">
					<span class="cd-text-disabled">Version</span>
					<span class="cd-mono">{versionInfo.version}</span>
				</div>
				{#if versionInfo.commit && versionInfo.commit !== 'unknown'}
					<div class="flex justify-between">
						<span class="cd-text-disabled">Commit</span>
						<span class="cd-mono text-xs">{versionInfo.commit}</span>
					</div>
				{/if}
				{#if versionInfo.buildDate && versionInfo.buildDate !== 'unknown'}
					<div class="flex justify-between">
						<span class="cd-text-disabled">Build Date</span>
						<span class="cd-mono text-xs">{versionInfo.buildDate}</span>
					</div>
				{/if}
			</div>
		{:else}
			<span class="text-sm cd-text-disabled">Loading version info...</span>
		{/if}
	</div>
</div>
