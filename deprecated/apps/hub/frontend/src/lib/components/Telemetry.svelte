<script lang="ts">
	import { Card } from '$lib/components/ui';
	import { telemetry } from '$lib/stores/telemetry';
	import type { TelemetryStatus, TelemetryData } from '$lib/types';
	import { Cpu, MonitorDot, MemoryStick, BatteryCharging, Zap, Fan, Gamepad2 } from 'lucide-svelte';

	let status = $state<TelemetryStatus>({ enabled: false, interval: 2 });
	let data = $state<TelemetryData | null>(null);

	// Subscribe to global stores (EventsOn listeners live in +page.svelte)
	$effect(() => {
		const unsubStatus = telemetry.status.subscribe((s) => (status = s));
		const unsubData = telemetry.data.subscribe((d) => (data = d));
		return () => {
			unsubStatus();
			unsubData();
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
							class="h-full rounded-full transition-[width] duration-300 {usageColor(data.cpu.usagePercent)}"
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
								class="h-full rounded-full transition-[width] duration-300 {usageColor(data.gpu.usagePercent)}"
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
							<span class="text-muted-foreground">Core Freq</span>
							<span class="font-mono">{formatFreq(data.gpu.freqMHz)}</span>
						</div>
					{/if}
					{#if data.gpu.memFreqMHz && data.gpu.memFreqMHz > 0}
						<div class="flex justify-between text-sm">
							<span class="text-muted-foreground">Mem Freq</span>
							<span class="font-mono">{formatFreq(data.gpu.memFreqMHz)}</span>
						</div>
					{/if}
					{#if data.gpu.vramTotalBytes && data.gpu.vramTotalBytes > 0}
						<div class="flex justify-between text-sm">
							<span class="text-muted-foreground">VRAM</span>
							<span class="font-mono">
								{formatBytes(data.gpu.vramUsedBytes ?? 0)} / {formatBytes(data.gpu.vramTotalBytes)}
							</span>
						</div>
						<div class="h-2 w-full rounded-full bg-secondary overflow-hidden">
							<div
								class="h-full rounded-full transition-[width] duration-300 {usageColor((data.gpu.vramUsedBytes ?? 0) / data.gpu.vramTotalBytes * 100)}"
								style="width: {Math.max(0, Math.min(100, (data.gpu.vramUsedBytes ?? 0) / data.gpu.vramTotalBytes * 100))}%"
							></div>
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
							class="h-full rounded-full transition-[width] duration-300 {usageColor(data.memory.usagePercent)}"
							style="width: {Math.max(0, Math.min(100, data.memory.usagePercent))}%"
						></div>
					</div>
					<div class="flex justify-between text-sm">
						<span class="text-muted-foreground">Used</span>
						<span class="font-mono">
							{formatBytes(data.memory.totalBytes - data.memory.availableBytes)} / {formatBytes(data.memory.totalBytes)}
						</span>
					</div>
					{#if data.memory.swapTotalBytes && data.memory.swapTotalBytes > 0}
						<div class="border-t border-border my-2 pt-2">
							<div class="flex justify-between text-sm mb-1">
								<span class="text-muted-foreground">Swap</span>
								<span class="font-mono">
									{formatBytes(data.memory.swapTotalBytes - (data.memory.swapFreeBytes ?? 0))} / {formatBytes(data.memory.swapTotalBytes)}
								</span>
							</div>
							<div class="h-2 w-full rounded-full bg-secondary overflow-hidden">
								<div
									class="h-full rounded-full transition-[width] duration-300 {usageColor((data.memory.swapTotalBytes - (data.memory.swapFreeBytes ?? 0)) / data.memory.swapTotalBytes * 100)}"
									style="width: {Math.max(0, Math.min(100, (data.memory.swapTotalBytes - (data.memory.swapFreeBytes ?? 0)) / data.memory.swapTotalBytes * 100))}%"
								></div>
							</div>
						</div>
					{/if}
				</div>
			</Card>
		{/if}

		<!-- System (Power + Battery + Fan) -->
		{#if data.power || data.battery || data.fan}
			<Card class="p-4">
				<div class="flex items-center gap-2 mb-3">
					<Zap class="w-5 h-5 text-primary" />
					<span class="font-medium text-sm">System</span>
				</div>
				<div class="space-y-2">
					{#if data.power}
						{#if data.power.powerWatts > 0}
							<div class="flex justify-between text-sm">
								<span class="text-muted-foreground">Power Draw</span>
								<span class="font-mono">{formatWatts(data.power.powerWatts)}</span>
							</div>
							{#if data.power.tdpWatts > 0}
								<div class="h-2 w-full rounded-full bg-secondary overflow-hidden">
									<div
										class="h-full rounded-full transition-[width] duration-300 {usageColor(data.power.powerWatts / data.power.tdpWatts * 100)}"
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
					{/if}
					{#if data.battery}
						{#if data.power}
							<div class="border-t border-border my-2 pt-2"></div>
						{/if}
						<div class="flex justify-between text-sm">
							<span class="text-muted-foreground">Battery</span>
							<span class="font-mono">{data.battery.capacity}%</span>
						</div>
						<div class="h-2 w-full rounded-full bg-secondary overflow-hidden">
							<div
								class="h-full rounded-full transition-[width] duration-300 {usageColor(100 - data.battery.capacity)}"
								style="width: {Math.max(0, Math.min(100, data.battery.capacity))}%"
							></div>
						</div>
						<div class="flex justify-between text-sm">
							<span class="text-muted-foreground">Status</span>
							<span class="font-mono">{data.battery.status}</span>
						</div>
					{/if}
					{#if data.fan}
						{#if data.power || data.battery}
							<div class="border-t border-border my-2 pt-2"></div>
						{/if}
						<div class="flex justify-between text-sm">
							<span class="text-muted-foreground">Fan</span>
							<span class="font-mono">{data.fan.rpm} RPM</span>
						</div>
					{/if}
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
