<script lang="ts">
	import { Button, Card, Input } from '$lib/components/ui';
	import { formatBytes } from '$lib/utils';
	import { toast } from '$lib/stores/toast';
	import { ExternalLink, Trash2, FolderOpen, Save, Loader2, HardDrive, Info, Server } from 'lucide-svelte';
	import {
		GetSteamGridDBAPIKey, SetSteamGridDBAPIKey,
		GetCacheSize, ClearImageCache, OpenCacheFolder,
		GetImageCacheEnabled, SetImageCacheEnabled,
		GetVersion,
		GetHubInfo, SetHubName
	} from '$lib/wailsjs';
	import { BrowserOpenURL } from '$wailsjs/runtime/runtime';
	import { browser } from '$app/environment';
	import type { VersionInfo } from '$lib/types';

	let hubName = $state('');
	let hubId = $state('');
	let hubPlatform = $state('');
	let savingHubName = $state(false);
	let apiKey = $state('');
	let cacheEnabled = $state(true);
	let cacheSize = $state('Calculating...');
	let saving = $state(false);
	let clearing = $state(false);
	let togglingCache = $state(false);
	let versionInfo = $state<VersionInfo | null>(null);

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
			cacheEnabled = await GetImageCacheEnabled();
		} catch (e) {
			console.error('Failed to load cache setting:', e);
			cacheEnabled = true;
		}

		try {
			versionInfo = await GetVersion();
		} catch (e) {
			console.error('Failed to load version:', e);
		}

		if (cacheEnabled) {
			await updateCacheSize();
		}
	}

	async function updateCacheSize() {
		if (!browser) return;
		try {
			const size = await GetCacheSize();
			cacheSize = formatBytes(size);
		} catch (e) {
			cacheSize = 'Unable to calculate';
		}
	}

	async function toggleCache() {
		togglingCache = true;
		try {
			const newValue = !cacheEnabled;
			await SetImageCacheEnabled(newValue);
			cacheEnabled = newValue;
			if (newValue) {
				await updateCacheSize();
				toast.success('Cache activado');
			} else {
				cacheSize = '0 B';
				toast.info('Cache desactivado', 'Las imagenes en cache fueron eliminadas');
			}
		} catch (e) {
			toast.error('Error', String(e));
		} finally {
			togglingCache = false;
		}
	}

	async function saveSettings() {
		saving = true;
		try {
			await SetSteamGridDBAPIKey(apiKey);
			toast.success('Configuracion guardada');
		} catch (e) {
			toast.error('Error al guardar', String(e));
		} finally {
			saving = false;
		}
	}

	async function clearCache() {
		clearing = true;
		try {
			await ClearImageCache();
			await updateCacheSize();
			toast.success('Cache limpiado');
		} catch (e) {
			toast.error('Error al limpiar cache', String(e));
		} finally {
			clearing = false;
		}
	}

	async function openCacheFolder() {
		try {
			await OpenCacheFolder();
		} catch (e) {
			toast.error('Error al abrir carpeta', String(e));
		}
	}

	async function saveHubName() {
		savingHubName = true;
		try {
			await SetHubName(hubName);
			toast.success('Nombre del Hub actualizado');
		} catch (e) {
			toast.error('Error al guardar', String(e));
		} finally {
			savingHubName = false;
		}
	}

	function openSteamGridDBApiPage() {
		BrowserOpenURL('https://www.steamgriddb.com/profile/preferences/api');
	}

	$effect(() => {
		if (!browser) return;
		loadSettings();
	});
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

	<!-- Image Cache Section -->
	<div class="cd-section p-4">
		<h3 class="cd-section-title">Image Cache</h3>
		<p class="text-sm cd-text-disabled mb-4">
			Cache images locally for faster loading. Disabling will delete all cached images.
		</p>

		<!-- Cache toggle -->
		<div class="flex items-center justify-between mb-4">
			<div class="flex items-center gap-3">
				<HardDrive class="w-5 h-5 cd-text-disabled" />
				<div>
					<span class="text-sm font-medium">Enable Local Cache</span>
					<p class="text-xs cd-text-disabled">Store downloaded images on disk</p>
				</div>
			</div>
			<button
				type="button"
				onclick={toggleCache}
				disabled={togglingCache}
				class="relative w-11 h-6 rounded-full transition-colors duration-200 focus:outline-none focus:ring-2 focus:ring-primary focus:ring-offset-2 focus:ring-offset-background {cacheEnabled ? 'bg-primary' : 'bg-muted'}"
			>
				<span
					class="absolute top-0.5 left-0.5 w-5 h-5 rounded-full bg-white shadow transition-transform duration-200 {cacheEnabled ? 'translate-x-5' : 'translate-x-0'}"
				></span>
			</button>
		</div>

		{#if cacheEnabled}
			<div class="flex items-center gap-4 mb-4">
				<span class="text-sm cd-text-disabled">Cache Size:</span>
				<span class="cd-mono">{cacheSize}</span>
			</div>

			<div class="flex gap-2">
				<Button variant="outline" onclick={clearCache} disabled={clearing}>
					{#if clearing}
						<Loader2 class="w-4 h-4 mr-2 animate-spin" />
					{:else}
						<Trash2 class="w-4 h-4 mr-2" />
					{/if}
					Clear Cache
				</Button>
				<Button variant="outline" onclick={openCacheFolder}>
					<FolderOpen class="w-4 h-4 mr-2" />
					Open Cache Folder
				</Button>
			</div>
		{/if}
	</div>

	<Button variant="gradient" onclick={saveSettings} disabled={saving} class="w-full">
		{#if saving}
			<Loader2 class="w-4 h-4 mr-2 animate-spin" />
		{:else}
			<Save class="w-4 h-4 mr-2" />
		{/if}
		Save Settings
	</Button>

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
