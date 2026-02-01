<script lang="ts">
	import { Button, Card, Input } from '$lib/components/ui';
	import { formatBytes } from '$lib/utils';
	import { ExternalLink, Trash2, FolderOpen, Save, Loader2 } from 'lucide-svelte';
	import {
		GetSteamGridDBAPIKey, SetSteamGridDBAPIKey,
		GetCacheSize, ClearImageCache, OpenCacheFolder
	} from '$lib/wailsjs';
	import { BrowserOpenURL } from '$wailsjs/runtime/runtime';
	import { browser } from '$app/environment';

	let apiKey = $state('');
	let cacheSize = $state('Calculating...');
	let saving = $state(false);
	let clearing = $state(false);

	async function loadSettings() {
		if (!browser) return;
		try {
			const key = await GetSteamGridDBAPIKey();
			apiKey = key || '';
		} catch (e) {
			console.error('Failed to load API key:', e);
		}

		await updateCacheSize();
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

	async function saveSettings() {
		saving = true;
		try {
			await SetSteamGridDBAPIKey(apiKey);
			alert('Settings saved successfully');
		} catch (e) {
			alert('Failed to save settings: ' + e);
		} finally {
			saving = false;
		}
	}

	async function clearCache() {
		if (!confirm('This will delete all cached SteamGridDB images.\nAre you sure?')) {
			return;
		}

		clearing = true;
		try {
			await ClearImageCache();
			await updateCacheSize();
			alert('Cache cleared');
		} catch (e) {
			alert('Failed to clear cache: ' + e);
		} finally {
			clearing = false;
		}
	}

	async function openCacheFolder() {
		try {
			await OpenCacheFolder();
		} catch (e) {
			alert('Failed to open cache folder: ' + e);
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

<div class="space-y-6 max-w-xl">
	<div>
		<h3 class="text-lg font-semibold mb-4">SteamGridDB Integration</h3>
		<p class="text-sm text-muted-foreground mb-4">
			SteamGridDB allows you to select custom artwork for your games.
		</p>
		<p class="text-sm mb-4">
			Get your API key from
			<button
				onclick={openSteamGridDBApiPage}
				class="text-blue-400 hover:underline inline-flex items-center gap-1"
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

	<hr class="border-border" />

	<div>
		<h3 class="text-lg font-semibold mb-4">Image Cache</h3>
		<p class="text-sm text-muted-foreground mb-4">
			Cached images are stored locally for faster loading.
		</p>

		<div class="flex items-center gap-4 mb-4">
			<span class="text-sm">Cache Size:</span>
			<span class="font-medium">{cacheSize}</span>
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
	</div>

	<hr class="border-border" />

	<Button onclick={saveSettings} disabled={saving}>
		{#if saving}
			<Loader2 class="w-4 h-4 mr-2 animate-spin" />
		{:else}
			<Save class="w-4 h-4 mr-2" />
		{/if}
		Save Settings
	</Button>
</div>
