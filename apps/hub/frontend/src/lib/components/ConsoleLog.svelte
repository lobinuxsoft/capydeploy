<script lang="ts">
	import { consolelog, consoleColors, type ConsoleColors, type ConsoleLogEntryWithId } from '$lib/stores/consolelog';
	import { EventsOn, SetConsoleLogFilter, SetConsoleLogEnabled } from '$lib/wailsjs';
	import { browser } from '$app/environment';
	import type { ConsoleLogStatus, ConsoleLogBatch } from '$lib/types';
	import {
		LOG_LEVEL_LOG,
		LOG_LEVEL_WARN,
		LOG_LEVEL_ERROR,
		LOG_LEVEL_INFO,
		LOG_LEVEL_DEBUG,
		LOG_LEVEL_DEFAULT
	} from '$lib/types';
	import { Terminal, Trash2, Power } from 'lucide-svelte';
	import { sanitizeCSS } from '$lib/console-format';
	import DropdownSelect from '$lib/components/ui/DropdownSelect.svelte';

	const levelToggles = [
		{ key: 'log', label: 'Log', bit: LOG_LEVEL_LOG },
		{ key: 'warn', label: 'Warn', bit: LOG_LEVEL_WARN },
		{ key: 'error', label: 'Error', bit: LOG_LEVEL_ERROR },
		{ key: 'info', label: 'Info', bit: LOG_LEVEL_INFO },
		{ key: 'debug', label: 'Debug', bit: LOG_LEVEL_DEBUG }
	];

	const sourceOptions = [
		{ value: 'all', label: 'All Sources' },
		{ value: 'console', label: 'Console' },
		{ value: 'game', label: 'Game' },
		{ value: 'network', label: 'Network' },
		{ value: 'javascript', label: 'JavaScript' },
		{ value: 'other', label: 'Other' }
	];

	let status = $state<ConsoleLogStatus>({ enabled: false, levelMask: LOG_LEVEL_DEFAULT });
	let entries = $state<ConsoleLogEntryWithId[]>([]);
	let totalDropped = $state<number>(0);
	let colors = $state<ConsoleColors>({ error: '#f87171', warn: '#facc15', info: '#60a5fa', debug: '#71717a', log: '#f1f5f9' });
	let levelMask = $state<number>(LOG_LEVEL_DEFAULT);

	// Filters
	let sourceFilter = $state('all');
	let searchText = $state('');

	// Auto-scroll
	let logContainer: HTMLDivElement | undefined = $state();
	let autoScroll = $state(true);

	// Subscribe to stores
	$effect(() => {
		const unsubStatus = consolelog.status.subscribe((s) => (status = s));
		const unsubEntries = consolelog.entries.subscribe((e) => (entries = e));
		const unsubDropped = consolelog.totalDropped.subscribe((d) => (totalDropped = d));
		const unsubColors = consoleColors.subscribe((c) => (colors = c));
		return () => {
			unsubStatus();
			unsubEntries();
			unsubDropped();
			unsubColors();
		};
	});

	// Listen for console log events from Wails
	$effect(() => {
		if (!browser) return;

		const unsubStatus = EventsOn('consolelog:status', (event: ConsoleLogStatus) => {
			consolelog.status.set(event);
			if (event.levelMask !== undefined) {
				levelMask = event.levelMask;
			}
		});

		const unsubData = EventsOn('consolelog:data', (event: ConsoleLogBatch) => {
			consolelog.addBatch(event.entries, event.dropped);
		});

		return () => {
			unsubStatus();
			unsubData();
		};
	});

	let enabling = $state(false);

	function handleEnableToggle(enabled: boolean) {
		enabling = true;
		SetConsoleLogEnabled(enabled)
			.catch((err: unknown) => console.error('Failed to toggle console log:', err))
			.finally(() => (enabling = false));
	}

	function handleToggle(bit: number) {
		levelMask = levelMask ^ bit;
		SetConsoleLogFilter(levelMask).catch((err: unknown) => {
			console.error('Failed to set console log filter:', err);
		});
	}

	// Auto-scroll to bottom when new entries arrive
	$effect(() => {
		const _ = entries.length;
		if (autoScroll && logContainer) {
			requestAnimationFrame(() => {
				if (logContainer) {
					logContainer.scrollTop = logContainer.scrollHeight;
				}
			});
		}
	});

	function handleScroll() {
		if (!logContainer) return;
		const { scrollTop, scrollHeight, clientHeight } = logContainer;
		autoScroll = scrollHeight - scrollTop - clientHeight < 50;
	}

	function levelBit(level: string): number {
		switch (level) {
			case 'log': return LOG_LEVEL_LOG;
			case 'warn': case 'warning': return LOG_LEVEL_WARN;
			case 'error': return LOG_LEVEL_ERROR;
			case 'info': return LOG_LEVEL_INFO;
			case 'debug': case 'verbose': return LOG_LEVEL_DEBUG;
			default: return 0;
		}
	}

	let filteredEntries = $derived(
		entries.filter((entry) => {
			const bit = levelBit(entry.level);
			if (bit !== 0 && (levelMask & bit) === 0) return false;
			if (sourceFilter !== 'all' && entry.source !== sourceFilter) return false;
			if (searchText && !entry.text.toLowerCase().includes(searchText.toLowerCase())) return false;
			return true;
		})
	);

	function formatTime(timestamp: number): string {
		const d = new Date(timestamp);
		return d.toLocaleTimeString('en-US', {
			hour12: false,
			hour: '2-digit',
			minute: '2-digit',
			second: '2-digit',
			fractionalSecondDigits: 3
		});
	}

	function levelColorStyle(level: string): string {
		switch (level) {
			case 'error': return `color: ${colors.error}`;
			case 'warning':
			case 'warn': return `color: ${colors.warn}`;
			case 'info': return `color: ${colors.info}`;
			case 'debug':
			case 'verbose': return `color: ${colors.debug}`;
			default: return `color: ${colors.log}`;
		}
	}

	function levelBadgeStyle(level: string): string {
		switch (level) {
			case 'error': return `background: ${colors.error}20; color: ${colors.error}`;
			case 'warning':
			case 'warn': return `background: ${colors.warn}20; color: ${colors.warn}`;
			case 'info': return `background: ${colors.info}20; color: ${colors.info}`;
			case 'debug':
			case 'verbose': return `background: ${colors.debug}20; color: ${colors.debug}`;
			default: return `background: ${colors.log}20; color: ${colors.log}`;
		}
	}
</script>

{#if !status.enabled && entries.length === 0}
	<div class="flex flex-col items-center justify-center py-16 text-center">
		<Terminal class="w-12 h-12 text-muted-foreground mb-4" />
		<p class="text-muted-foreground text-sm max-w-md mb-4">
			Enable console log streaming from the agent to view Steam CEF logs,
			or enable a game log wrapper to capture game output.
		</p>
		<button
			type="button"
			onclick={() => handleEnableToggle(true)}
			disabled={enabling}
			class="text-sm bg-primary text-primary-foreground rounded px-4 py-2 hover:bg-primary/90 transition-colors disabled:opacity-50 flex items-center gap-2"
		>
			<Power class="w-4 h-4" />
			{enabling ? 'Enabling...' : 'Enable Console Log'}
		</button>
	</div>
{:else if status.enabled && entries.length === 0}
	<div class="flex flex-col items-center justify-center py-16 text-center">
		<div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary mb-4"></div>
		<p class="text-muted-foreground text-sm">Waiting for console log data...</p>
	</div>
{:else}
	<!-- Toolbar -->
	<div class="flex items-center gap-2 mb-3 flex-wrap">
		<!-- Level toggle buttons -->
		<div class="flex items-center gap-0.5">
			{#each levelToggles as toggle (toggle.key)}
				<button
					type="button"
					onmousedown={(e) => { e.preventDefault(); handleToggle(toggle.bit); }}
					class="text-[10px] font-medium uppercase px-2 py-0.5 rounded border cursor-pointer select-none transition-all {levelMask & toggle.bit ? 'opacity-100 bg-primary/25 border-primary text-primary' : 'opacity-50 bg-secondary border-border text-muted-foreground hover:opacity-75'}"
				>
					{toggle.label}
				</button>
			{/each}
		</div>

		<DropdownSelect options={sourceOptions} bind:value={sourceFilter} />

		<input
			type="text"
			placeholder="Search..."
			bind:value={searchText}
			class="text-xs bg-secondary border border-border rounded px-2 py-1.5 text-foreground flex-1 min-w-[120px]"
		/>

		<button
			type="button"
			onclick={() => consolelog.clear()}
			class="text-xs bg-secondary border border-border rounded px-2 py-1.5 text-muted-foreground hover:text-foreground transition-colors flex items-center gap-1"
		>
			<Trash2 class="w-3 h-3" />
			Clear
		</button>

		{#if status.enabled}
			<button
				type="button"
				onclick={() => handleEnableToggle(false)}
				disabled={enabling}
				class="text-xs bg-destructive/10 border border-destructive/30 rounded px-2 py-1.5 text-destructive hover:bg-destructive/20 transition-colors flex items-center gap-1 disabled:opacity-50"
			>
				<Power class="w-3 h-3" />
				Disable
			</button>
		{:else}
			<button
				type="button"
				onclick={() => handleEnableToggle(true)}
				disabled={enabling}
				class="text-xs bg-primary/10 border border-primary/30 rounded px-2 py-1.5 text-primary hover:bg-primary/20 transition-colors flex items-center gap-1 disabled:opacity-50"
			>
				<Power class="w-3 h-3" />
				Enable Console
			</button>
		{/if}

		{#if totalDropped > 0}
			<span class="text-xs text-yellow-400">{totalDropped} dropped</span>
		{/if}

		<span class="text-xs text-muted-foreground ml-auto">
			{filteredEntries.length}/{entries.length}
		</span>
	</div>

	<!-- Log entries -->
	<div
		bind:this={logContainer}
		onscroll={handleScroll}
		class="bg-zinc-950 rounded border border-border overflow-auto font-mono text-xs leading-relaxed"
		style="max-height: 500px;"
	>
		{#each filteredEntries as entry (entry.id)}
			<div class="flex gap-2 px-3 py-0.5 hover:bg-zinc-900/50 border-b border-zinc-900">
				<span class="text-zinc-600 shrink-0">{formatTime(entry.timestamp)}</span>
				<span class="shrink-0 px-1 rounded text-[10px] font-medium uppercase" style={levelBadgeStyle(entry.level)}>
					{entry.level}
				</span>
				{#if entry.segments && entry.segments.length > 0}
					<span class="break-all">{#each entry.segments as seg}{#if seg.css}<span style={sanitizeCSS(seg.css)}>{seg.text}</span>{:else}<span style={levelColorStyle(entry.level)}>{seg.text}</span>{/if}{/each}</span>
				{:else}
					<span class="break-all" style={levelColorStyle(entry.level)}>{entry.text}</span>
				{/if}
			</div>
		{/each}

		{#if !autoScroll}
			<button
				type="button"
				onclick={() => {
					autoScroll = true;
					if (logContainer) logContainer.scrollTop = logContainer.scrollHeight;
				}}
				class="sticky bottom-2 left-1/2 -translate-x-1/2 bg-primary text-primary-foreground text-xs px-3 py-1 rounded-full shadow-lg"
			>
				Scroll to bottom
			</button>
		{/if}
	</div>
{/if}
