<script lang="ts">
	import { Card } from '$lib/components/ui';
	import { telemetry } from '$lib/stores/telemetry';
	import { EventsOn, EventsOff } from '$lib/wailsjs';
	import { browser } from '$app/environment';
	import type { TelemetryStatus, TelemetryData } from '$lib/types';
	import { Cpu, MonitorDot, MemoryStick, BatteryCharging, Zap, Fan, Gamepad2 } from 'lucide-svelte';

	let status = $state<TelemetryStatus>({ enabled: false, interval: 2 });
	let data = $state<TelemetryData | null>(null);

	// Subscribe to stores
	$effect(() => {
		const unsubStatus = telemetry.status.subscribe((s) => (status = s));
		const unsubData = telemetry.data.subscribe((d) => (data = d));
		return () => {
			unsubStatus();
			unsubData();
		};
	});

	// Listen for telemetry events from Wails
	$effect(() => {
		if (!browser) return;

		EventsOn('telemetry:status', (event: TelemetryStatus) => {
			telemetry.status.set(event);
		});

		EventsOn('telemetry:data', (event: TelemetryData) => {
			telemetry.data.set(event);
		});

		return () => {
			EventsOff('telemetry:status');
			EventsOff('telemetry:data');
		};
	});

	function usageColor(percent: number): string {
		if (percent < 0) return 'bg-muted';
		if (percent < 60) return 'bg-green-500';
		if (percent < 85) return 'bg-yellow-500';
		return 'bg-red-500';
	}

	function tempColor(temp: number): string {
		if (temp < 0) return 'text-muted-foreground';
		if (temp < 60) return 'text-green-500';
		if (temp < 80) return 'text-yellow-500';
		return 'text-red-500';
	}

	function formatBytes(bytes: number): string {
		if (bytes < 0) return 'N/A';
		const gb = bytes / (1024 * 1024 * 1024);
		if (gb >= 1) return `${gb.toFixed(1)} GB`;
		const mb = bytes / (1024 * 1024);
		return `${mb.toFixed(0)} MB`;
	}

	function formatTemp(temp: number): string {
		if (temp < 0) return 'N/A';
		return `${temp.toFixed(0)}\u00B0C`;
	}

	function formatPercent(val: number): string {
		if (val < 0) return 'N/A';
		return `${val.toFixed(1)}%`;
	}

	function formatFreq(mhz: number): string {
		if (mhz < 0) return 'N/A';
		if (mhz >= 1000) return `${(mhz / 1000).toFixed(2)} GHz`;
		return `${mhz.toFixed(0)} MHz`;
	}

	function formatWatts(watts: number): string {
		if (watts < 0) return 'N/A';
		return `${watts.toFixed(1)} W`;
	}
</script>

{#if !status.enabled}
	<div class="flex flex-col items-center justify-center py-16 text-center">
		<Cpu class="w-12 h-12 text-muted-foreground mb-4" />
		<p class="text-muted-foreground text-sm max-w-md">
			Enable telemetry sending from the agent to view hardware metrics.
		</p>
	</div>
{:else if !data}
	<div class="flex flex-col items-center justify-center py-16 text-center">
		<div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary mb-4"></div>
		<p class="text-muted-foreground text-sm">Waiting for telemetry data...</p>
	</div>
{:else}
	<div class="grid grid-cols-1 md:grid-cols-2 gap-4">
		<!-- CPU -->
		{#if data.cpu}
			<Card class="p-4">
				<div class="flex items-center gap-2 mb-3">
					<Cpu class="w-5 h-5 text-primary" />
					<span class="font-medium text-sm">CPU</span>
				</div>
				<div class="space-y-2">
					<div class="flex justify-between text-sm">
						<span class="text-muted-foreground">Usage</span>
						<span class="font-mono">{formatPercent(data.cpu.usagePercent)}</span>
					</div>
					<div class="h-2 w-full rounded-full bg-secondary overflow-hidden">
						<div
							class="h-full rounded-full transition-all duration-300 {usageColor(data.cpu.usagePercent)}"
							style="width: {Math.max(0, Math.min(100, data.cpu.usagePercent))}%"
						></div>
					</div>
					{#if data.cpu.tempCelsius >= 0}
						<div class="flex justify-between text-sm">
							<span class="text-muted-foreground">Temperature</span>
							<span class="font-mono {tempColor(data.cpu.tempCelsius)}">{formatTemp(data.cpu.tempCelsius)}</span>
						</div>
					{/if}
					{#if data.cpu.freqMHz >= 0}
						<div class="flex justify-between text-sm">
							<span class="text-muted-foreground">Frequency</span>
							<span class="font-mono">{formatFreq(data.cpu.freqMHz)}</span>
						</div>
					{/if}
				</div>
			</Card>
		{/if}

		<!-- GPU -->
		{#if data.gpu}
			<Card class="p-4">
				<div class="flex items-center gap-2 mb-3">
					<MonitorDot class="w-5 h-5 text-primary" />
					<span class="font-medium text-sm">GPU</span>
				</div>
				<div class="space-y-2">
					{#if data.gpu.usagePercent >= 0}
						<div class="flex justify-between text-sm">
							<span class="text-muted-foreground">Usage</span>
							<span class="font-mono">{formatPercent(data.gpu.usagePercent)}</span>
						</div>
						<div class="h-2 w-full rounded-full bg-secondary overflow-hidden">
							<div
								class="h-full rounded-full transition-all duration-300 {usageColor(data.gpu.usagePercent)}"
								style="width: {Math.max(0, Math.min(100, data.gpu.usagePercent))}%"
							></div>
						</div>
					{/if}
					{#if data.gpu.tempCelsius >= 0}
						<div class="flex justify-between text-sm">
							<span class="text-muted-foreground">Temperature</span>
							<span class="font-mono {tempColor(data.gpu.tempCelsius)}">{formatTemp(data.gpu.tempCelsius)}</span>
						</div>
					{/if}
					{#if data.gpu.freqMHz >= 0}
						<div class="flex justify-between text-sm">
							<span class="text-muted-foreground">Frequency</span>
							<span class="font-mono">{formatFreq(data.gpu.freqMHz)}</span>
						</div>
					{/if}
				</div>
			</Card>
		{/if}

		<!-- Memory -->
		{#if data.memory}
			<Card class="p-4">
				<div class="flex items-center gap-2 mb-3">
					<MemoryStick class="w-5 h-5 text-primary" />
					<span class="font-medium text-sm">Memory</span>
				</div>
				<div class="space-y-2">
					<div class="flex justify-between text-sm">
						<span class="text-muted-foreground">Usage</span>
						<span class="font-mono">{formatPercent(data.memory.usagePercent)}</span>
					</div>
					<div class="h-2 w-full rounded-full bg-secondary overflow-hidden">
						<div
							class="h-full rounded-full transition-all duration-300 {usageColor(data.memory.usagePercent)}"
							style="width: {Math.max(0, Math.min(100, data.memory.usagePercent))}%"
						></div>
					</div>
					<div class="flex justify-between text-sm">
						<span class="text-muted-foreground">Used</span>
						<span class="font-mono">
							{formatBytes(data.memory.totalBytes - data.memory.availableBytes)} / {formatBytes(data.memory.totalBytes)}
						</span>
					</div>
				</div>
			</Card>
		{/if}

		<!-- Power (TDP) -->
		{#if data.power}
			<Card class="p-4">
				<div class="flex items-center gap-2 mb-3">
					<Zap class="w-5 h-5 text-primary" />
					<span class="font-medium text-sm">Power</span>
				</div>
				<div class="space-y-2">
					{#if data.power.powerWatts > 0}
						<div class="flex justify-between text-sm">
							<span class="text-muted-foreground">Draw</span>
							<span class="font-mono">{formatWatts(data.power.powerWatts)}</span>
						</div>
						{#if data.power.tdpWatts > 0}
							<div class="h-2 w-full rounded-full bg-secondary overflow-hidden">
								<div
									class="h-full rounded-full transition-all duration-300 {usageColor(data.power.powerWatts / data.power.tdpWatts * 100)}"
									style="width: {Math.max(0, Math.min(100, data.power.powerWatts / data.power.tdpWatts * 100))}%"
								></div>
							</div>
						{/if}
					{/if}
					{#if data.power.tdpWatts > 0}
						<div class="flex justify-between text-sm">
							<span class="text-muted-foreground">TDP Limit</span>
							<span class="font-mono">{formatWatts(data.power.tdpWatts)}</span>
						</div>
					{/if}
				</div>
			</Card>
		{/if}

		<!-- Battery -->
		{#if data.battery}
			<Card class="p-4">
				<div class="flex items-center gap-2 mb-3">
					<BatteryCharging class="w-5 h-5 text-primary" />
					<span class="font-medium text-sm">Battery</span>
				</div>
				<div class="space-y-2">
					<div class="flex justify-between text-sm">
						<span class="text-muted-foreground">Capacity</span>
						<span class="font-mono">{data.battery.capacity}%</span>
					</div>
					<div class="h-2 w-full rounded-full bg-secondary overflow-hidden">
						<div
							class="h-full rounded-full transition-all duration-300 {usageColor(100 - data.battery.capacity)}"
							style="width: {Math.max(0, Math.min(100, data.battery.capacity))}%"
						></div>
					</div>
					<div class="flex justify-between text-sm">
						<span class="text-muted-foreground">Status</span>
						<span class="font-mono">{data.battery.status}</span>
					</div>
				</div>
			</Card>
		{/if}

		<!-- Fan -->
		{#if data.fan}
			<Card class="p-4">
				<div class="flex items-center gap-2 mb-3">
					<Fan class="w-5 h-5 text-primary" />
					<span class="font-medium text-sm">Fan</span>
				</div>
				<div class="space-y-2">
					<div class="flex justify-between text-sm">
						<span class="text-muted-foreground">Speed</span>
						<span class="font-mono">{data.fan.rpm} RPM</span>
					</div>
				</div>
			</Card>
		{/if}

		<!-- Steam -->
		{#if data.steam}
			<Card class="p-4">
				<div class="flex items-center gap-2 mb-3">
					<Gamepad2 class="w-5 h-5 text-primary" />
					<span class="font-medium text-sm">Steam</span>
				</div>
				<div class="space-y-2">
					<div class="flex justify-between text-sm">
						<span class="text-muted-foreground">Status</span>
						<span class="font-mono {data.steam.running ? 'text-green-500' : 'text-muted-foreground'}">
							{data.steam.running ? 'Running' : 'Not Running'}
						</span>
					</div>
					{#if data.steam.gamingMode}
						<div class="flex justify-between text-sm">
							<span class="text-muted-foreground">Mode</span>
							<span class="font-mono text-primary">Gaming Mode</span>
						</div>
					{/if}
				</div>
			</Card>
		{/if}
	</div>
{/if}
