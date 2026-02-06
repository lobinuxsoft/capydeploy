<script lang="ts">
	import { Button, Card, Input } from '$lib/components/ui';
	import { toast } from '$lib/stores/toast';
	import { ExternalLink, Save, Loader2, Info, Server } from 'lucide-svelte';
	import {
		GetSteamGridDBAPIKey, SetSteamGridDBAPIKey,
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
	let saving = $state(false);
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
			versionInfo = await GetVersion();
		} catch (e) {
			console.error('Failed to load version:', e);
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
