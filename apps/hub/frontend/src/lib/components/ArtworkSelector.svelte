<script lang="ts">
	import { Button, Input } from '$lib/components/ui';
	import FiltersModal from '$lib/components/FiltersModal.svelte';
	import type {
		ArtworkSelection, SearchResult, GridData, ImageData, ImageFilters
	} from '$lib/types';
	import { Search, X, Loader2, RefreshCw, Filter, Check } from 'lucide-svelte';
	import { cn } from '$lib/utils';
	import { SearchGames, GetGrids, GetHeroes, GetLogos, GetIcons } from '$lib/wailsjs';
	import { browser } from '$app/environment';
	import { connectionStatus } from '$lib/stores/connection';

	interface Props {
		gameName: string;
		currentSelection: ArtworkSelection | null;
		onsave: (selection: ArtworkSelection) => void;
		onclose: () => void;
	}

	let { gameName, currentSelection, onsave, onclose }: Props = $props();

	let searchQuery = $state(gameName);
	let searchResults = $state<SearchResult[]>([]);
	let selectedGameID = $state(currentSelection?.gridDBGameID || 0);
	let selectedGameName = $state('');
	let searching = $state(false);
	let loading = $state(false);
	let statusMessage = $state('Search for a game to select artwork');
	let activeTab = $state<'capsule' | 'wide' | 'hero' | 'logo' | 'icon'>('capsule');

	// Selection state - separate variables for better reactivity
	let gridDBGameID = $state(currentSelection?.gridDBGameID || 0);
	let gridPortrait = $state(currentSelection?.gridPortrait || '');
	let gridLandscape = $state(currentSelection?.gridLandscape || '');
	let heroImage = $state(currentSelection?.heroImage || '');
	let logoImage = $state(currentSelection?.logoImage || '');
	let iconImage = $state(currentSelection?.iconImage || '');

	// Image data
	let capsules = $state<GridData[]>([]);
	let wideCapsules = $state<GridData[]>([]);
	let heroes = $state<ImageData[]>([]);
	let logos = $state<ImageData[]>([]);
	let icons = $state<ImageData[]>([]);

	// Filters - per tab
	const defaultFilters: ImageFilters = {
		style: '',
		mimeType: '',
		dimension: '',
		imageType: '',
		showNsfw: false,
		showHumor: true
	};
	let filters = $state<ImageFilters>({ ...defaultFilters });

	// Pages
	let capsulePage = $state(0);
	let widePage = $state(0);
	let heroPage = $state(0);
	let logoPage = $state(0);
	let iconPage = $state(0);

	// Has more
	let hasMoreCapsules = $state(false);
	let hasMoreWide = $state(false);
	let hasMoreHeroes = $state(false);
	let hasMoreLogos = $state(false);
	let hasMoreIcons = $state(false);

	// Show filters modal
	let showFiltersModal = $state(false);

	// Check if any filters are active
	let hasActiveFilters = $derived(
		filters.style !== '' ||
		filters.mimeType !== '' ||
		filters.dimension !== '' ||
		filters.imageType !== '' ||
		filters.showNsfw !== false ||
		filters.showHumor !== true
	);

	// Check if thumb URL is animated (SteamGridDB uses .webm for animated thumbs)
	function isAnimatedThumb(thumb: string): boolean {
		return thumb?.includes('.webm') || false;
	}

	// Cleanup function to clear all cached data
	function clearCache() {
		capsules = [];
		wideCapsules = [];
		heroes = [];
		logos = [];
		icons = [];
	}

	const tabs: { id: 'capsule' | 'wide' | 'hero' | 'logo' | 'icon'; label: string }[] = [
		{ id: 'capsule', label: 'Capsule' },
		{ id: 'wide', label: 'Wide' },
		{ id: 'hero', label: 'Hero' },
		{ id: 'logo', label: 'Logo' },
		{ id: 'icon', label: 'Icon' }
	];

	// Handle filter changes from modal
	function handleFiltersApply(newFilters: ImageFilters) {
		filters = newFilters;
		reloadCurrentTab();
	}

	// Handle tab change - reload with current filters
	function handleTabChange(tabId: typeof activeTab) {
		if (tabId === activeTab) return;
		activeTab = tabId;
		// Reload the tab with current filters if a game is selected
		if (selectedGameID) {
			reloadCurrentTab();
		}
	}

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

		// Load all image types
		await Promise.all([
			loadCapsules(false),
			loadWideCapsules(false),
			loadHeroes(false),
			loadLogos(false),
			loadIcons(false)
		]);
	}

	async function loadCapsules(append: boolean) {
		if (!selectedGameID) return;
		if (!append) {
			capsulePage = 0;
			capsules = [];
		}
		loading = true;
		statusMessage = 'Loading capsules...';
		try {
			const grids = await GetGrids(selectedGameID, filters, capsulePage);
			const portraits = (grids || []).filter((g: any) => g.height > g.width);
			capsules = append ? [...capsules, ...portraits] : portraits;
			hasMoreCapsules = (grids || []).length >= 50;
			const animCount = portraits.filter((p: any) => isAnimatedThumb(p.thumb)).length;
			capsulePage++;
			statusMessage = `Loaded ${portraits.length} capsules (${animCount} animated)`;
		} catch (e) {
			console.error('LoadCapsules error:', e);
			statusMessage = `Error: ${e}`;
		} finally {
			loading = false;
		}
	}

	async function loadWideCapsules(append: boolean) {
		if (!selectedGameID) return;
		if (!append) {
			widePage = 0;
			wideCapsules = [];
		}
		loading = true;
		statusMessage = 'Loading wide capsules...';
		try {
			const grids = await GetGrids(selectedGameID, filters, widePage);
			const landscapes = (grids || []).filter((g: any) => g.width > g.height);
			wideCapsules = append ? [...wideCapsules, ...landscapes] : landscapes;
			hasMoreWide = (grids || []).length >= 50;
			const animCount = landscapes.filter((p: any) => isAnimatedThumb(p.thumb)).length;
			widePage++;
			statusMessage = `Loaded ${landscapes.length} wide capsules (${animCount} animated)`;
		} catch (e) {
			statusMessage = `Error: ${e}`;
		} finally {
			loading = false;
		}
	}

	async function loadHeroes(append: boolean) {
		if (!selectedGameID) return;
		if (!append) {
			heroPage = 0;
			heroes = [];
		}
		loading = true;
		statusMessage = 'Loading heroes...';
		try {
			const data = await GetHeroes(selectedGameID, filters, heroPage);
			const items = data || [];
			heroes = append ? [...heroes, ...items] : items;
			hasMoreHeroes = items.length >= 50;
			const animCount = items.filter((p: any) => isAnimatedThumb(p.thumb)).length;
			heroPage++;
			statusMessage = `Loaded ${items.length} heroes (${animCount} animated)`;
		} catch (e) {
			statusMessage = `Error: ${e}`;
		} finally {
			loading = false;
		}
	}

	async function loadLogos(append: boolean) {
		if (!selectedGameID) return;
		if (!append) {
			logoPage = 0;
			logos = [];
		}
		loading = true;
		statusMessage = 'Loading logos...';
		try {
			const data = await GetLogos(selectedGameID, filters, logoPage);
			const items = data || [];
			logos = append ? [...logos, ...items] : items;
			hasMoreLogos = items.length >= 50;
			logoPage++;
			statusMessage = `Loaded ${items.length} logos`;
		} catch (e) {
			statusMessage = `Error: ${e}`;
		} finally {
			loading = false;
		}
	}

	async function loadIcons(append: boolean) {
		if (!selectedGameID) return;
		if (!append) {
			iconPage = 0;
			icons = [];
		}
		loading = true;
		statusMessage = 'Loading icons...';
		try {
			const data = await GetIcons(selectedGameID, filters, iconPage);
			const items = data || [];
			icons = append ? [...icons, ...items] : items;
			hasMoreIcons = items.length >= 50;
			iconPage++;
			statusMessage = `Loaded ${items.length} icons`;
		} catch (e) {
			statusMessage = `Error: ${e}`;
		} finally {
			loading = false;
		}
	}

	function reloadCurrentTab() {
		switch (activeTab) {
			case 'capsule': loadCapsules(false); break;
			case 'wide': loadWideCapsules(false); break;
			case 'hero': loadHeroes(false); break;
			case 'logo': loadLogos(false); break;
			case 'icon': loadIcons(false); break;
		}
	}

	function selectCapsule(img: GridData) {
		gridPortrait = img.url;
	}

	function selectWide(img: GridData) {
		gridLandscape = img.url;
	}

	function selectHero(img: ImageData) {
		heroImage = img.url;
	}

	function selectLogo(img: ImageData) {
		logoImage = img.url;
	}

	function selectIcon(img: ImageData) {
		iconImage = img.url;
	}

	function clearAll() {
		gridPortrait = '';
		gridLandscape = '';
		heroImage = '';
		logoImage = '';
		iconImage = '';
	}

	function handleSave() {
		onsave({
			gridDBGameID,
			gridPortrait,
			gridLandscape,
			heroImage,
			logoImage,
			iconImage
		});
	}

	// Check if an image is selected
	function isSelected(url: string, type: 'capsule' | 'wide' | 'hero' | 'logo' | 'icon'): boolean {
		switch (type) {
			case 'capsule': return gridPortrait === url;
			case 'wide': return gridLandscape === url;
			case 'hero': return heroImage === url;
			case 'logo': return logoImage === url;
			case 'icon': return iconImage === url;
			default: return false;
		}
	}

	// Auto-search on mount if gameName is provided
	$effect(() => {
		if (!browser) return;
		if (gameName && !currentSelection?.gridDBGameID) {
			searchGames();
		}

		// Cleanup on component destroy
		return () => {
			clearCache();
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
		<div class="w-56 border-r flex flex-col shrink-0">
			<div class="p-3 space-y-2 shrink-0">
				<h3 class="font-semibold text-sm gradient-text">Search SteamGridDB</h3>
				<div class="flex gap-1">
					<Input
						bind:value={searchQuery}
						placeholder="Game name..."
						class="text-sm"
						onkeydown={(e) => e.key === 'Enter' && searchGames()}
					/>
					<Button size="icon" onclick={searchGames} disabled={searching}>
						{#if searching}
							<Loader2 class="w-4 h-4 animate-spin" />
						{:else}
							<Search class="w-4 h-4" />
						{/if}
					</Button>
				</div>
				{#if selectedGameName}
					<p class="text-xs text-green-500 truncate">
						{selectedGameName}
					</p>
				{/if}
			</div>
			<div class="flex-1 overflow-y-auto min-h-0">
				{#each searchResults as game}
					<button
						type="button"
						class={cn(
							'w-full p-2 text-left hover:bg-accent border-b text-xs',
							selectedGameID === game.id && 'bg-accent'
						)}
						onclick={() => selectGame(game)}
					>
						<div class="font-medium truncate">{game.name}</div>
						{#if game.verified}
							<span class="text-[10px] text-green-500">[Verified]</span>
						{/if}
					</button>
				{/each}
			</div>
		</div>

		<!-- Center panel: Images -->
		<div class="flex-1 flex flex-col min-h-0 min-w-0">
			<!-- Tabs -->
			<div class="flex items-center gap-1 p-2 border-b shrink-0">
				{#each tabs as tab}
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
				<Button variant="ghost" size="sm" onclick={reloadCurrentTab} disabled={loading || !selectedGameID}>
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
				{#if activeTab === 'capsule'}
					<div class="text-xs text-muted-foreground mb-2">600x900 - Portrait capsule</div>
					<div class="grid grid-cols-4 gap-3">
						{#each capsules as img (img.url)}
							{@const isAnim = isAnimatedThumb(img.thumb)}
							{@const selected = isSelected(img.url, 'capsule')}
							<button
								type="button"
								class={cn(
									'relative rounded-lg overflow-hidden border-2 transition-all',
									selected ? 'border-green-500 ring-2 ring-green-500/50' : 'border-transparent hover:border-blue-500'
								)}
								onclick={() => selectCapsule(img)}
							>
								{#if isAnim}
									<video
										src={img.thumb}
										class="w-full aspect-[2/3] object-cover bg-muted"
										muted
										loop
										playsinline
										autoplay
									></video>
								{:else}
									<img
										src={img.thumb || img.url}
										alt=""
										class="w-full aspect-[2/3] object-cover bg-muted"
									/>
								{/if}
								{#if selected}
									<div class="absolute top-1 right-1 bg-green-500 rounded-full p-0.5">
										<Check class="w-3 h-3 text-white" />
									</div>
								{/if}
								{#if isAnim}
									<span class="absolute top-1 left-1 z-10 bg-orange-500 text-white text-[9px] px-1 rounded font-bold shadow">ANIM</span>
								{/if}
								<div class="absolute bottom-0 left-0 right-0 bg-black/70 text-white text-[9px] p-0.5 text-center">
									{img.width}x{img.height}
								</div>
							</button>
						{/each}
					</div>
					{#if capsules.length === 0 && !loading && selectedGameID}
						<div class="text-center text-muted-foreground py-8 text-sm">No capsules found</div>
					{/if}
					{#if hasMoreCapsules}
						<div class="text-center py-3">
							<Button variant="outline" size="sm" onclick={() => loadCapsules(true)} disabled={loading}>
								Load More
							</Button>
						</div>
					{/if}
				{:else if activeTab === 'wide'}
					<div class="text-xs text-muted-foreground mb-2">920x430 - Wide capsule</div>
					<div class="grid grid-cols-2 gap-3">
						{#each wideCapsules as img (img.url)}
							{@const isAnim = isAnimatedThumb(img.thumb)}
							{@const selected = isSelected(img.url, 'wide')}
							<button
								type="button"
								class={cn(
									'relative rounded-lg overflow-hidden border-2 transition-all',
									selected ? 'border-green-500 ring-2 ring-green-500/50' : 'border-transparent hover:border-blue-500'
								)}
								onclick={() => selectWide(img)}
							>
								{#if isAnim}
									<video
										src={img.thumb}
										class="w-full aspect-[460/215] object-cover bg-muted"
										muted
										loop
										playsinline
										autoplay
									></video>
								{:else}
									<img
										src={img.thumb || img.url}
										alt=""
										class="w-full aspect-[460/215] object-cover bg-muted"
									/>
								{/if}
								{#if selected}
									<div class="absolute top-1 right-1 bg-green-500 rounded-full p-0.5">
										<Check class="w-3 h-3 text-white" />
									</div>
								{/if}
								{#if isAnim}
									<span class="absolute top-1 left-1 z-10 bg-orange-500 text-white text-[9px] px-1 rounded font-bold shadow">ANIM</span>
								{/if}
								<div class="absolute bottom-0 left-0 right-0 bg-black/70 text-white text-[9px] p-0.5 text-center">
									{img.width}x{img.height}
								</div>
							</button>
						{/each}
					</div>
					{#if wideCapsules.length === 0 && !loading && selectedGameID}
						<div class="text-center text-muted-foreground py-8 text-sm">No wide capsules found</div>
					{/if}
					{#if hasMoreWide}
						<div class="text-center py-3">
							<Button variant="outline" size="sm" onclick={() => loadWideCapsules(true)} disabled={loading}>
								Load More
							</Button>
						</div>
					{/if}
				{:else if activeTab === 'hero'}
					<div class="text-xs text-muted-foreground mb-2">1920x620 - Hero banner</div>
					<div class="grid grid-cols-2 gap-3">
						{#each heroes as img (img.url)}
							{@const isAnim = isAnimatedThumb(img.thumb)}
							{@const selected = isSelected(img.url, 'hero')}
							<button
								type="button"
								class={cn(
									'relative rounded-lg overflow-hidden border-2 transition-all',
									selected ? 'border-green-500 ring-2 ring-green-500/50' : 'border-transparent hover:border-blue-500'
								)}
								onclick={() => selectHero(img)}
							>
								{#if isAnim}
									<video
										src={img.thumb}
										class="w-full aspect-[1920/620] object-cover bg-muted"
										muted
										loop
										playsinline
										autoplay
									></video>
								{:else}
									<img
										src={img.thumb || img.url}
										alt=""
										class="w-full aspect-[1920/620] object-cover bg-muted"
									/>
								{/if}
								{#if selected}
									<div class="absolute top-1 right-1 bg-green-500 rounded-full p-0.5">
										<Check class="w-3 h-3 text-white" />
									</div>
								{/if}
								{#if isAnim}
									<span class="absolute top-1 left-1 z-10 bg-orange-500 text-white text-[9px] px-1 rounded font-bold shadow">ANIM</span>
								{/if}
								<div class="absolute bottom-0 left-0 right-0 bg-black/70 text-white text-[9px] p-0.5 text-center">
									{img.width}x{img.height}
								</div>
							</button>
						{/each}
					</div>
					{#if heroes.length === 0 && !loading && selectedGameID}
						<div class="text-center text-muted-foreground py-8 text-sm">No heroes found</div>
					{/if}
					{#if hasMoreHeroes}
						<div class="text-center py-3">
							<Button variant="outline" size="sm" onclick={() => loadHeroes(true)} disabled={loading}>
								Load More
							</Button>
						</div>
					{/if}
				{:else if activeTab === 'logo'}
					<div class="text-xs text-muted-foreground mb-2">Game logo (transparent)</div>
					<div class="grid grid-cols-4 gap-3">
						{#each logos as img (img.url)}
							{@const isAnim = isAnimatedThumb(img.thumb)}
							{@const selected = isSelected(img.url, 'logo')}
							<button
								type="button"
								class={cn(
									'relative rounded-lg overflow-hidden border-2 transition-all bg-muted p-1',
									selected ? 'border-green-500 ring-2 ring-green-500/50' : 'border-transparent hover:border-blue-500'
								)}
								onclick={() => selectLogo(img)}
							>
								{#if isAnim}
									<video
										src={img.thumb}
										class="w-full aspect-square object-contain"
										muted
										loop
										playsinline
										autoplay
									></video>
								{:else}
									<img
										src={img.thumb || img.url}
										alt=""
										class="w-full aspect-square object-contain"
									/>
								{/if}
								{#if selected}
									<div class="absolute top-1 right-1 bg-green-500 rounded-full p-0.5">
										<Check class="w-3 h-3 text-white" />
									</div>
								{/if}
								{#if isAnim}
									<span class="absolute top-1 left-1 z-10 bg-orange-500 text-white text-[9px] px-1 rounded font-bold shadow">ANIM</span>
								{/if}
								<div class="text-[9px] text-center text-muted-foreground">
									{img.width}x{img.height}
								</div>
							</button>
						{/each}
					</div>
					{#if logos.length === 0 && !loading && selectedGameID}
						<div class="text-center text-muted-foreground py-8 text-sm">No logos found</div>
					{/if}
					{#if hasMoreLogos}
						<div class="text-center py-3">
							<Button variant="outline" size="sm" onclick={() => loadLogos(true)} disabled={loading}>
								Load More
							</Button>
						</div>
					{/if}
				{:else if activeTab === 'icon'}
					<div class="text-xs text-muted-foreground mb-2">Square icon</div>
					<div class="grid grid-cols-6 gap-3">
						{#each icons as img (img.url)}
							{@const isAnim = isAnimatedThumb(img.thumb)}
							{@const selected = isSelected(img.url, 'icon')}
							<button
								type="button"
								class={cn(
									'relative rounded-lg overflow-hidden border-2 transition-all bg-muted p-0.5',
									selected ? 'border-green-500 ring-2 ring-green-500/50' : 'border-transparent hover:border-blue-500'
								)}
								onclick={() => selectIcon(img)}
							>
								{#if isAnim}
									<video
										src={img.thumb}
										class="w-full aspect-square object-contain"
										muted
										loop
										playsinline
										autoplay
									></video>
								{:else}
									<img
										src={img.thumb || img.url}
										alt=""
										class="w-full aspect-square object-contain"
									/>
								{/if}
								{#if selected}
									<div class="absolute top-0.5 right-0.5 bg-green-500 rounded-full p-0.5">
										<Check class="w-2 h-2 text-white" />
									</div>
								{/if}
								{#if isAnim}
									<span class="absolute top-0.5 left-0.5 z-10 bg-orange-500 text-white text-[7px] px-0.5 rounded font-bold shadow">ANIM</span>
								{/if}
								<div class="text-[8px] text-center text-muted-foreground">
									{img.width}
								</div>
							</button>
						{/each}
					</div>
					{#if icons.length === 0 && !loading && selectedGameID}
						<div class="text-center text-muted-foreground py-8 text-sm">No icons found</div>
					{/if}
					{#if hasMoreIcons}
						<div class="text-center py-3">
							<Button variant="outline" size="sm" onclick={() => loadIcons(true)} disabled={loading}>
								Load More
							</Button>
						</div>
					{/if}
				{/if}

				{#if !selectedGameID}
					<div class="text-center text-muted-foreground py-8">
						Search and select a game to browse artwork
					</div>
				{/if}
			</div>
		</div>

		<!-- Right panel: Selected Artwork -->
		<div class="w-56 border-l flex flex-col shrink-0">
			<div class="p-3 shrink-0">
				<h3 class="font-semibold text-sm mb-3 gradient-text">Selected Artwork</h3>
				<div class="space-y-3">
					<!-- Capsule -->
					<div class="flex items-center gap-3">
						<span class="w-12 text-xs text-muted-foreground shrink-0">Capsule</span>
						{#if gridPortrait}
							<img src={gridPortrait} alt="Capsule" class="h-14 w-auto rounded border-2 border-green-500 object-contain" />
						{:else}
							<div class="h-14 w-10 rounded border border-dashed border-muted-foreground/30 flex items-center justify-center">
								<span class="text-[10px] text-muted-foreground">—</span>
							</div>
						{/if}
					</div>
					<!-- Wide -->
					<div class="flex items-center gap-3">
						<span class="w-12 text-xs text-muted-foreground shrink-0">Wide</span>
						{#if gridLandscape}
							<img src={gridLandscape} alt="Wide" class="h-10 w-auto rounded border-2 border-green-500 object-contain" />
						{:else}
							<div class="h-10 w-20 rounded border border-dashed border-muted-foreground/30 flex items-center justify-center">
								<span class="text-[10px] text-muted-foreground">—</span>
							</div>
						{/if}
					</div>
					<!-- Hero -->
					<div class="flex items-center gap-3">
						<span class="w-12 text-xs text-muted-foreground shrink-0">Hero</span>
						{#if heroImage}
							<img src={heroImage} alt="Hero" class="h-8 w-auto rounded border-2 border-green-500 object-contain" />
						{:else}
							<div class="h-8 w-24 rounded border border-dashed border-muted-foreground/30 flex items-center justify-center">
								<span class="text-[10px] text-muted-foreground">—</span>
							</div>
						{/if}
					</div>
					<!-- Logo -->
					<div class="flex items-center gap-3">
						<span class="w-12 text-xs text-muted-foreground shrink-0">Logo</span>
						{#if logoImage}
							<img src={logoImage} alt="Logo" class="h-10 w-auto rounded border-2 border-green-500 object-contain bg-muted/50" />
						{:else}
							<div class="h-10 w-16 rounded border border-dashed border-muted-foreground/30 flex items-center justify-center">
								<span class="text-[10px] text-muted-foreground">—</span>
							</div>
						{/if}
					</div>
					<!-- Icon -->
					<div class="flex items-center gap-3">
						<span class="w-12 text-xs text-muted-foreground shrink-0">Icon</span>
						{#if iconImage}
							<img src={iconImage} alt="Icon" class="h-10 w-10 rounded border-2 border-green-500 object-contain bg-muted/50" />
						{:else}
							<div class="h-10 w-10 rounded border border-dashed border-muted-foreground/30 flex items-center justify-center">
								<span class="text-[10px] text-muted-foreground">—</span>
							</div>
						{/if}
					</div>
				</div>
			</div>
		</div>
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
