<script lang="ts">
	import { Button, Card, Dialog, Input } from '$lib/components/ui';
	import { devices } from '$lib/stores/devices';
	import { connectionStatus } from '$lib/stores/connection';
	import type { DeviceConfig, NetworkDevice } from '$lib/types';
	import { Monitor, LogIn, LogOut, Pencil, Trash2, Search, Plus, Loader2 } from 'lucide-svelte';
	import { cn } from '$lib/utils';
	import {
		GetDevices, AddDevice, UpdateDevice, RemoveDevice,
		ConnectDevice, DisconnectDevice, GetConnectionStatus, ScanNetwork
	} from '$lib/wailsjs';

	let showDeviceForm = $state(false);
	let showScanDialog = $state(false);
	let editingDevice: DeviceConfig | null = $state(null);
	let connecting = $state<string | null>(null);
	let scanning = $state(false);
	let foundDevices = $state<NetworkDevice[]>([]);
	let selectedNetDevice = $state<NetworkDevice | null>(null);
	let scanError = $state('');

	// Form state
	let formName = $state('');
	let formHost = $state('');
	let formPort = $state('22');
	let formUser = $state('deck');
	let formPassword = $state('');
	let formKeyFile = $state('');
	let authMethod = $state<'password' | 'key'>('password');

	async function loadDevices() {
		try {
			const list = await GetDevices();
			devices.set(list || []);
		} catch (e) {
			console.error('Failed to load devices:', e);
		}
	}

	async function loadConnectionStatus() {
		try {
			const status = await GetConnectionStatus();
			connectionStatus.set(status);
		} catch (e) {
			console.error('Failed to get connection status:', e);
		}
	}

	$effect(() => {
		loadDevices();
		loadConnectionStatus();
	});

	function resetForm() {
		formName = '';
		formHost = '';
		formPort = '22';
		formUser = 'deck';
		formPassword = '';
		formKeyFile = '';
		authMethod = 'password';
		editingDevice = null;
	}

	function openAddForm(ip = '', hostname = '') {
		resetForm();
		if (ip) formHost = ip;
		if (hostname) formName = hostname;
		showDeviceForm = true;
	}

	function openEditForm(device: DeviceConfig) {
		editingDevice = device;
		formName = device.name;
		formHost = device.host;
		formPort = String(device.port);
		formUser = device.user;
		formPassword = device.password || '';
		formKeyFile = device.key_file || '';
		authMethod = device.key_file ? 'key' : 'password';
		showDeviceForm = true;
	}

	async function saveDevice() {
		const device: DeviceConfig = {
			name: formName || formHost,
			host: formHost,
			port: parseInt(formPort) || 22,
			user: formUser,
			password: authMethod === 'password' ? formPassword : '',
			key_file: authMethod === 'key' ? formKeyFile : ''
		};

		try {
			if (editingDevice) {
				await UpdateDevice(editingDevice.host, device);
			} else {
				await AddDevice(device);
			}
			await loadDevices();
			showDeviceForm = false;
			resetForm();
		} catch (e) {
			console.error('Failed to save device:', e);
			alert('Error: ' + e);
		}
	}

	async function deleteDevice(host: string) {
		if (!confirm('Are you sure you want to delete this device?')) return;
		try {
			await RemoveDevice(host);
			await loadDevices();
			await loadConnectionStatus();
		} catch (e) {
			console.error('Failed to delete device:', e);
		}
	}

	async function connect(host: string) {
		connecting = host;
		try {
			await ConnectDevice(host);
			await loadConnectionStatus();
		} catch (e) {
			console.error('Failed to connect:', e);
			alert('Connection failed: ' + e);
		} finally {
			connecting = null;
		}
	}

	async function disconnect() {
		try {
			await DisconnectDevice();
			await loadConnectionStatus();
		} catch (e) {
			console.error('Failed to disconnect:', e);
		}
	}

	async function scanNetworkHandler() {
		scanning = true;
		foundDevices = [];
		scanError = '';
		try {
			const results = await ScanNetwork();
			foundDevices = results || [];
		} catch (e) {
			console.error('Scan failed:', e);
			scanError = String(e);
		} finally {
			scanning = false;
		}
	}

	function selectAndConfigureDevice() {
		if (selectedNetDevice) {
			showScanDialog = false;
			openAddForm(selectedNetDevice.ip, selectedNetDevice.hostname);
		}
	}
</script>

<div class="space-y-4">
	<div class="flex gap-2">
		<Button onclick={() => showScanDialog = true}>
			<Search class="w-4 h-4 mr-2" />
			Scan Network
		</Button>
		<Button onclick={() => openAddForm()}>
			<Plus class="w-4 h-4 mr-2" />
			Add Device
		</Button>
	</div>

	<div class="space-y-2">
		{#each $devices as device}
			{@const isConnected = $connectionStatus.connected && $connectionStatus.host === device.host}
			<Card class="p-4">
				<div class="flex items-center justify-between">
					<div class="flex items-center gap-3">
						<div class="relative">
							<Monitor class="w-6 h-6" />
							<div
								class={cn(
									'absolute -bottom-0.5 -right-0.5 w-2.5 h-2.5 rounded-full border border-background',
									isConnected ? 'bg-green-500' : 'bg-gray-500'
								)}
							></div>
						</div>
						<div>
							<div class="font-medium">
								{device.name} ({device.user}@{device.host})
							</div>
							<div class="text-sm text-muted-foreground">
								{isConnected ? 'Connected' : 'Disconnected'}
							</div>
						</div>
					</div>
					<div class="flex gap-1">
						{#if isConnected}
							<Button variant="destructive" size="icon" onclick={disconnect}>
								<LogOut class="w-4 h-4" />
							</Button>
						{:else}
							<Button size="icon" onclick={() => connect(device.host)} disabled={connecting === device.host}>
								{#if connecting === device.host}
									<Loader2 class="w-4 h-4 animate-spin" />
								{:else}
									<LogIn class="w-4 h-4" />
								{/if}
							</Button>
						{/if}
						<Button variant="ghost" size="icon" onclick={() => openEditForm(device)}>
							<Pencil class="w-4 h-4" />
						</Button>
						<Button variant="ghost" size="icon" onclick={() => deleteDevice(device.host)}>
							<Trash2 class="w-4 h-4" />
						</Button>
					</div>
				</div>
			</Card>
		{/each}

		{#if $devices.length === 0}
			<div class="text-center text-muted-foreground py-8">
				No devices configured. Add a device or scan your network.
			</div>
		{/if}
	</div>
</div>

<!-- Device Form Dialog -->
<Dialog bind:open={showDeviceForm} title={editingDevice ? 'Edit Device' : 'Add Device'}>
	<div class="space-y-4">
		<div class="space-y-2">
			<label class="text-sm font-medium">Name</label>
			<Input bind:value={formName} placeholder="My Bazzite Device" />
		</div>
		<div class="space-y-2">
			<label class="text-sm font-medium">Host/IP</label>
			<Input bind:value={formHost} placeholder="192.168.1.100" />
		</div>
		<div class="grid grid-cols-2 gap-4">
			<div class="space-y-2">
				<label class="text-sm font-medium">Port</label>
				<Input bind:value={formPort} placeholder="22" />
			</div>
			<div class="space-y-2">
				<label class="text-sm font-medium">User</label>
				<Input bind:value={formUser} placeholder="deck" />
			</div>
		</div>

		<div class="space-y-2">
			<label class="text-sm font-medium">Authentication Method</label>
			<div class="flex gap-4">
				<label class="flex items-center gap-2 cursor-pointer">
					<input type="radio" bind:group={authMethod} value="password" class="accent-primary" />
					Password
				</label>
				<label class="flex items-center gap-2 cursor-pointer">
					<input type="radio" bind:group={authMethod} value="key" class="accent-primary" />
					SSH Key
				</label>
			</div>
		</div>

		{#if authMethod === 'password'}
			<div class="space-y-2">
				<label class="text-sm font-medium">Password</label>
				<Input type="password" bind:value={formPassword} placeholder="SSH Password" />
			</div>
		{:else}
			<div class="space-y-2">
				<label class="text-sm font-medium">SSH Key Path</label>
				<Input bind:value={formKeyFile} placeholder="~/.ssh/id_ed25519" />
			</div>
		{/if}

		<div class="flex justify-end gap-2 pt-4">
			<Button variant="outline" onclick={() => { showDeviceForm = false; resetForm(); }}>
				Cancel
			</Button>
			<Button onclick={saveDevice}>
				Save
			</Button>
		</div>
	</div>
</Dialog>

<!-- Network Scan Dialog -->
<Dialog bind:open={showScanDialog} title="Scan Network" class="max-w-xl">
	<div class="space-y-4">
		<div class="flex gap-2">
			<Button onclick={scanNetworkHandler} disabled={scanning}>
				{#if scanning}
					<Loader2 class="w-4 h-4 mr-2 animate-spin" />
					Scanning...
				{:else}
					<Search class="w-4 h-4 mr-2" />
					Scan
				{/if}
			</Button>
			<Button
				onclick={selectAndConfigureDevice}
				disabled={!selectedNetDevice}
			>
				Select & Configure
			</Button>
		</div>

		<div class="text-sm text-muted-foreground">
			{#if scanning}
				Scanning network for devices with SSH (port 22)...
			{:else if scanError}
				<span class="text-red-500">Error: {scanError}</span>
			{:else if foundDevices.length > 0}
				Found {foundDevices.length} device(s) with SSH
			{:else}
				Click 'Scan' to find devices on your network...
			{/if}
		</div>

		<div class="border rounded-md max-h-64 overflow-y-auto">
			{#each foundDevices as device}
				<button
					type="button"
					class={cn(
						'w-full flex items-center gap-3 p-3 hover:bg-accent text-left border-b last:border-b-0',
						selectedNetDevice?.ip === device.ip && 'bg-accent'
					)}
					onclick={() => selectedNetDevice = device}
				>
					<Monitor class="w-5 h-5 text-green-500" />
					<div>
						<div class="font-medium">{device.ip}</div>
						{#if device.hostname}
							<div class="text-sm text-muted-foreground">{device.hostname}</div>
						{/if}
					</div>
					{#if device.hasSSH}
						<span class="ml-auto text-xs text-green-500">SSH</span>
					{/if}
				</button>
			{:else}
				{#if !scanning}
					<div class="p-4 text-center text-muted-foreground">
						No devices found. Click Scan to search.
					</div>
				{/if}
			{/each}
		</div>
	</div>
</Dialog>
