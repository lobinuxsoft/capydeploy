<script lang="ts">
	import { consolelog, consoleColors, type ConsoleColors } from '$lib/stores/consolelog';
	import { EventsOn } from '$lib/wailsjs';
	import { browser } from '$app/environment';
	import type { ConsoleLogStatus, ConsoleLogEntry, ConsoleLogBatch } from '$lib/types';
	import { Terminal, Trash2 } from 'lucide-svelte';
	import { sanitizeCSS } from '$lib/console-format';
	import { DropdownSelect } from '$lib/components/ui';

	const levelOptions = [
		{ value: 'all', label: 'All Levels' },
		{ value: 'log', label: 'Log' },
		{ value: 'warn', label: 'Warn' },
		{ value: 'error', label: 'Error' },
		{ value: 'info', label: 'Info' }
	];

	const sourceOptions = [
		{ value: 'all', label: 'All Sources' },
		{ value: 'console', label: 'Console' },
		{ value: 'network', label: 'Network' },
		{ value: 'javascript', label: 'JavaScript' },
		{ value: 'other', label: 'Other' }
	];

	let status = $state<ConsoleLogStatus>({ enabled: false });
	let entries = $state<ConsoleLogEntry[]>([]);
	let totalDropped = $state<number>(0);
	let colors = $state<ConsoleColors>({ error: '#f87171', warn: '#facc15', info: '#60a5fa', debug: '#71717a', log: '#f1f5f9' });

	// Filters
	let levelFilter = $state('all');
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
		});

		const unsubData = EventsOn('consolelog:data', (event: ConsoleLogBatch) => {
			consolelog.addBatch(event.entries, event.dropped);
		});

		return () => {
			unsubStatus();
			unsubData();
		};
	});

	// Auto-scroll to bottom when new entries arrive
	$effect(() => {
		// Track entries length to trigger effect
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
		// Auto-scroll if within 50px of bottom
		autoScroll = scrollHeight - scrollTop - clientHeight < 50;
	}

	let filteredEntries = $derived(
		entries.filter((entry) => {
			if (levelFilter !== 'all' && entry.level !== levelFilter) return false;
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

{#if !status.enabled}
	<div class="flex flex-col items-center justify-center py-16 text-center">
		<Terminal class="w-12 h-12 text-muted-foreground mb-4" />
		<p class="text-muted-foreground text-sm max-w-md">
			Enable console log streaming from the agent to view Steam CEF logs.
		</p>
	</div>
{:else if entries.length === 0}
	<div class="flex flex-col items-center justify-center py-16 text-center">
		<div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary mb-4"></div>
		<p class="text-muted-foreground text-sm">Waiting for console log data...</p>
	</div>
{:else}
	<!-- Toolbar -->
	<div class="flex items-center gap-2 mb-3 flex-wrap">
		<DropdownSelect options={levelOptions} bind:value={levelFilter} />
		<DropdownSelect options={sourceOptions} bind:value={sourceFilter} />

		<input
			type="text"
			placeholder="Search..."
			bind:value={searchText}
			class="text-xs bg-secondary border border-border rounded px-2 py-1.5 text-foreground flex-1 min-w-[120px]"
		/>

		<button
			onclick={() => consolelog.clear()}
			class="text-xs bg-secondary border border-border rounded px-2 py-1.5 text-muted-foreground hover:text-foreground transition-colors flex items-center gap-1"
		>
			<Trash2 class="w-3 h-3" />
			Clear
		</button>

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
		{#each filteredEntries as entry (entry.timestamp + entry.text)}
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
