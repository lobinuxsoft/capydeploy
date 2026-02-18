<script lang="ts">
	import { Button } from '$lib/components/ui';
	import FiltersModal from '$lib/components/FiltersModal.svelte';
	import ArtworkSearch from './artwork/ArtworkSearch.svelte';
	import ArtworkGrid from './artwork/ArtworkGrid.svelte';
	import ArtworkPreview from './artwork/ArtworkPreview.svelte';
	import {
		TAB_CONFIGS, createArtworkTab,
		type ArtworkTabId, type ArtworkTabState
	} from './artwork/artworkTab.svelte';
	import type {
		ArtworkSelection, ArtworkFileResult, SearchResult, GridData, ImageData, ImageFilters
	} from '$lib/types';
	import { X, RefreshCw, Filter, Upload } from 'lucide-svelte';
	import { cn } from '$lib/utils';
	import { SearchGames, GetGrids, GetHeroes, GetLogos, GetIcons, SelectArtworkFile, GetArtworkPreview } from '$lib/wailsjs';
	import { browser } from '$app/environment';
	import { connectionStatus } from '$lib/stores/connection';

	interface Props {
		gameName: string;
		currentSelection: ArtworkSelection | null;
		onsave: (selection: ArtworkSelection) => void;
		onclose: () => void;
	}

	let { gameName, currentSelection, onsave, onclose }: Props = $props();

	// --- Search state ---
	let searchQuery = $state(gameName);
	let searchResults = $state<SearchResult[]>([]);
	let selectedGameID = $state(currentSelection?.gridDBGameID || 0);
	let selectedGameName = $state('');
	let searching = $state(false);
	let loading = $state(false);
	let statusMessage = $state('Search for a game to select artwork');
	let activeTab = $state<ArtworkTabId>('capsule');

	// --- Filters ---
	const defaultFilters: ImageFilters = {
		style: '', mimeType: '', dimension: '', imageType: '',
		showNsfw: false, showHumor: true
	};
	let filters = $state<ImageFilters>({ ...defaultFilters });
	let showFiltersModal = $state(false);

	let hasActiveFilters = $derived(
		filters.style !== '' || filters.mimeType !== '' ||
		filters.dimension !== '' || filters.imageType !== '' ||
		filters.showNsfw !== false || filters.showHumor !== true
	);

	// --- Tab states (composable) ---
	const tabStates: Record<ArtworkTabId, ArtworkTabState> = {
		capsule: createArtworkTab(TAB_CONFIGS[0], GetGrids, (items) =>
			(items as GridData[]).filter((g) => g.height > g.width)
		),
		wide: createArtworkTab(TAB_CONFIGS[1], GetGrids, (items) =>
			(items as GridData[]).filter((g) => g.width > g.height)
		),
		hero: createArtworkTab(TAB_CONFIGS[2], GetHeroes),
		logo: createArtworkTab(TAB_CONFIGS[3], GetLogos),
		icon: createArtworkTab(TAB_CONFIGS[4], GetIcons)
	};

	// Initialize selections from currentSelection
	if (currentSelection) {
		tabStates.capsule.selectedUrl = currentSelection.gridPortrait || '';
		tabStates.wide.selectedUrl = currentSelection.gridLandscape || '';
		tabStates.hero.selectedUrl = currentSelection.heroImage || '';
		tabStates.logo.selectedUrl = currentSelection.logoImage || '';
		tabStates.icon.selectedUrl = currentSelection.iconImage || '';
	}

	let gridDBGameID = $state(currentSelection?.gridDBGameID || 0);
	const currentTab = $derived(tabStates[activeTab]);
	const currentConfig = $derived(TAB_CONFIGS.find((t) => t.id === activeTab)!);

	// --- Local file previews ---
	let localPreviews = $state<Record<string, string>>({});

	function previewUrl(url: string): string {
		if (url.startsWith('file://')) return localPreviews[url] || '';
		return url;
	}

	function isLocalFile(url: string): boolean {
		return url.startsWith('file://');
	}

	// --- Preview sidebar data (config-driven from TAB_CONFIGS) ---
	const previewSelections = $derived(
		TAB_CONFIGS.map((cfg) => ({
			label: cfg.label,
			url: tabStates[cfg.id].selectedUrl,
			imgClass: cfg.preview.imgClass,
			placeholderClass: cfg.preview.placeholderClass
		}))
	);

	// --- Actions ---

	async function searchGames() {
		if (!searchQuery.trim()) return;
		searching = true;
		statusMessage = 'Searching...';
		try {
			searchResults = await SearchGames(searchQuery);
			statusMessage = `Found ${searchResults.length} games`;
		} catch (e) {
			statusMessage = `Search error: ${e}`;
		} finally {
			searching = false;
		}
	}

	async function selectGame(game: SearchResult) {
		selectedGameID = game.id;
		selectedGameName = game.name;
		gridDBGameID = game.id;
		loading = true;
		try {
			const results = await Promise.all(
				(Object.keys(tabStates) as ArtworkTabId[]).map((id) =>
					tabStates[id].load(game.id, filters, false)
				)
			);
			statusMessage = results.filter(Boolean).join(' | ');
		} catch (e) {
			statusMessage = `Error: ${e}`;
		} finally {
			loading = false;
		}
	}

	async function loadTab(append = false) {
		loading = true;
		try {
			statusMessage = await currentTab.load(selectedGameID, filters, append);
		} catch (e) {
			statusMessage = `Error: ${e}`;
		} finally {
			loading = false;
		}
	}

	function handleTabChange(tabId: ArtworkTabId) {
		if (tabId === activeTab) return;
		activeTab = tabId;
		if (selectedGameID) loadTab();
	}

	function handleFiltersApply(newFilters: ImageFilters) {
		filters = newFilters;
		loadTab();
	}

	function handleSelect(img: GridData | ImageData) {
		currentTab.selectedUrl = img.url;
	}

	async function handleUploadLocal() {
		try {
			const result: ArtworkFileResult | null = await SelectArtworkFile();
			if (!result) return;
			const fileUrl = `file://${result.path}`;
			currentTab.selectedUrl = fileUrl;
			localPreviews[fileUrl] = result.dataURI;
			statusMessage = `Local image selected: ${result.path}`;
		} catch (e) {
			statusMessage = `Error: ${e}`;
		}
	}

	function clearAll() {
		for (const id of Object.keys(tabStates) as ArtworkTabId[]) {
			tabStates[id].selectedUrl = '';
		}
	}

	function handleSave() {
		onsave({
			gridDBGameID,
			gridPortrait: tabStates.capsule.selectedUrl,
			gridLandscape: tabStates.wide.selectedUrl,
			heroImage: tabStates.hero.selectedUrl,
			logoImage: tabStates.logo.selectedUrl,
			iconImage: tabStates.icon.selectedUrl
		});
	}

	// --- Effects ---

	$effect(() => {
		if (!browser) return;
		const allUrls = (Object.keys(tabStates) as ArtworkTabId[]).map((id) => tabStates[id].selectedUrl);
		for (const url of allUrls) {
			if (url.startsWith('file://') && !localPreviews[url]) {
				const localPath = url.slice(7);
				GetArtworkPreview(localPath).then((dataURI) => {
					localPreviews[url] = dataURI;
				}).catch(() => {});
			}
		}
	});

	$effect(() => {
		if (!browser) return;
		if (gameName && !currentSelection?.gridDBGameID) {
			searchGames();
		}
		return () => {
			for (const id of Object.keys(tabStates) as ArtworkTabId[]) {
				tabStates[id].reset();
			}
		};
	});
</script>

<!-- Full screen overlay dialog -->
<div class="fixed inset-0 z-50 bg-background flex flex-col h-screen">
	<!-- Header -->
	<div class="flex items-center justify-between p-3 border-b shrink-0">
		<h2 class="text-lg font-semibold gradient-text">Select Artwork - {gameName}</h2>
		<Button variant="ghost" size="icon" onclick={onclose}>
			<X class="w-5 h-5" />
		</Button>
	</div>

	<!-- Main content -->
	<div class="flex-1 flex min-h-0">
		<!-- Left panel: Search -->
		<ArtworkSearch
			bind:searchQuery
			{searching}
			{searchResults}
			{selectedGameID}
			{selectedGameName}
			onsearch={searchGames}
			onselectgame={selectGame}
		/>

		<!-- Center panel: Images -->
		<div class="flex-1 flex flex-col min-h-0 min-w-0">
			<!-- Tabs + toolbar -->
			<div class="flex items-center gap-1 p-2 border-b shrink-0">
				{#each TAB_CONFIGS as tab}
					<button
						type="button"
						onclick={() => handleTabChange(tab.id)}
						class={cn(
							'px-3 py-1.5 text-sm rounded-md transition-colors',
							activeTab === tab.id
								? 'bg-primary text-primary-foreground'
								: 'hover:bg-accent'
						)}
					>
						{tab.label}
					</button>
				{/each}
				<div class="flex-1"></div>
				<Button variant="outline" size="sm" onclick={handleUploadLocal}>
					<Upload class="w-4 h-4 mr-1" />
					Local
				</Button>
				<Button
					variant={hasActiveFilters ? 'default' : 'ghost'}
					size="sm"
					onclick={() => showFiltersModal = true}
					disabled={!selectedGameID}
				>
					<Filter class="w-4 h-4 mr-1" />
					Filters
					{#if hasActiveFilters}
						<span class="ml-1 px-1.5 py-0.5 text-[10px] bg-white/20 rounded">ON</span>
					{/if}
				</Button>
				<Button variant="ghost" size="sm" onclick={loadTab} disabled={loading || !selectedGameID}>
					<RefreshCw class={cn('w-4 h-4', loading && 'animate-spin')} />
				</Button>
			</div>

			<!-- Filters status bar -->
			{#if hasActiveFilters}
				<div class="px-3 py-1.5 border-b bg-primary/10 text-xs text-muted-foreground flex items-center gap-2">
					<Filter class="w-3 h-3" />
					<span>Some assets may be hidden due to active filters</span>
					<button
						type="button"
						class="ml-auto text-primary hover:underline"
						onclick={() => showFiltersModal = true}
					>
						Edit filters
					</button>
				</div>
			{/if}

			<!-- Image grid -->
			<div class="flex-1 overflow-y-auto p-2 min-h-0">
				<ArtworkGrid
					config={currentConfig}
					items={currentTab.items}
					selectedUrl={currentTab.selectedUrl}
					hasMore={currentTab.hasMore}
					{loading}
					{selectedGameID}
					onselect={handleSelect}
					onloadmore={() => loadTab(true)}
				/>
				{#if !selectedGameID}
					<div class="text-center text-muted-foreground py-8">
						Search and select a game to browse artwork
					</div>
				{/if}
			</div>
		</div>

		<!-- Right panel: Selected Artwork -->
		<ArtworkPreview
			selections={previewSelections}
			{previewUrl}
			{isLocalFile}
		/>
	</div>

	<!-- Footer -->
	<div class="p-3 border-t flex items-center justify-between shrink-0">
		<p class="text-xs text-muted-foreground truncate flex-1 mr-4">{statusMessage}</p>
		<div class="flex gap-2 shrink-0">
			<Button variant="outline" size="sm" onclick={onclose}>Cancel</Button>
			<Button variant="outline" size="sm" onclick={clearAll}>Clear All</Button>
			<Button size="sm" onclick={handleSave}>Save Selection</Button>
		</div>
	</div>
</div>

<!-- Filters Modal -->
<FiltersModal
	bind:open={showFiltersModal}
	assetType={activeTab}
	{filters}
	supportedFormats={$connectionStatus.supportedImageFormats}
	onapply={handleFiltersApply}
/>
