<script lang="ts">
	import { Button, Input, Select, Checkbox } from '$lib/components/ui';
	import type {
		ArtworkSelection, SearchResult, GridData, ImageData, ImageFilters
	} from '$lib/types';
	import {
		gridStyles, heroStyles, logoStyles, iconStyles,
		capsuleDimensions, wideCapsuleDimensions, heroDimensions, logoDimensions, iconDimensions,
		gridMimes, logoMimes, iconMimes, animationOptions
	} from '$lib/types';
	import { isAnimatedImage } from '$lib/utils';
	import { Search, X, ExternalLink, Loader2, RefreshCw, Filter, Check, ImageOff } from 'lucide-svelte';
	import { cn } from '$lib/utils';
	import { SearchGames, GetGrids, GetHeroes, GetLogos, GetIcons, ProxyImage } from '$lib/wailsjs';

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
	let activeTab = $state('capsule');

	// Selection state - separate variables for better reactivity
	let gridDBGameID = $state(currentSelection?.gridDBGameID || 0);
	let gridPortrait = $state(currentSelection?.gridPortrait || '');
	let gridLandscape = $state(currentSelection?.gridLandscape || '');
	let heroImage = $state(currentSelection?.heroImage || '');
	let logoImage = $state(currentSelection?.logoImage || '');
	let iconImage = $state(currentSelection?.iconImage || '');

	// Preview
	let previewUrl = $state('');
	let previewInfo = $state('');

	// Image data
	let capsules = $state<GridData[]>([]);
	let wideCapsules = $state<GridData[]>([]);
	let heroes = $state<ImageData[]>([]);
	let logos = $state<ImageData[]>([]);
	let icons = $state<ImageData[]>([]);

	// Filters - separate for each tab for better control
	let filterStyle = $state('');
	let filterMime = $state('');
	let filterDimension = $state('');
	let filterAnimation = $state('');
	let filterNsfw = $state(false);
	let filterHumor = $state(true);

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

	// Show filters panel
	let showFilters = $state(false);

	// Image proxy cache - maps original URL to data URL
	let imageCache = $state<Map<string, string>>(new Map());
	let loadingImages = $state<Set<string>>(new Set());

	const tabs = [
		{ id: 'capsule', label: 'Capsule' },
		{ id: 'wide', label: 'Wide' },
		{ id: 'hero', label: 'Hero' },
		{ id: 'logo', label: 'Logo' },
		{ id: 'icon', label: 'Icon' }
	];

	function getStyleOptions(): string[] {
		switch (activeTab) {
			case 'capsule':
			case 'wide': return gridStyles;
			case 'hero': return heroStyles;
			case 'logo': return logoStyles;
			case 'icon': return iconStyles;
			default: return gridStyles;
		}
	}

	function getDimensionOptions(): string[] {
		switch (activeTab) {
			case 'capsule': return capsuleDimensions;
			case 'wide': return wideCapsuleDimensions;
			case 'hero': return heroDimensions;
			case 'logo': return logoDimensions;
			case 'icon': return iconDimensions;
			default: return capsuleDimensions;
		}
	}

	function getMimeOptions(): string[] {
		switch (activeTab) {
			case 'logo': return logoMimes;
			case 'icon': return iconMimes;
			default: return gridMimes;
		}
	}

	function getCurrentFilters(): ImageFilters {
		return {
			style: filterStyle,
			mimeType: filterMime,
			dimension: filterDimension,
			imageType: filterAnimation,
			showNsfw: filterNsfw,
			showHumor: filterHumor
		};
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
			const grids = await GetGrids(selectedGameID, getCurrentFilters(), capsulePage);
			const portraits = (grids || []).filter((g: any) => g.height > g.width);
			capsules = append ? [...capsules, ...portraits] : portraits;
			hasMoreCapsules = (grids || []).length >= 50;
			const animCount = portraits.filter((p: any) => isAnimatedImage(p.mime, p.url)).length;
			statusMessage = `Loading ${portraits.length} capsule images...`;
			capsulePage++;

			// Preload images through proxy
			await preloadImages(portraits);
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
			const grids = await GetGrids(selectedGameID, getCurrentFilters(), widePage);
			const landscapes = (grids || []).filter((g: any) => g.width > g.height);
			wideCapsules = append ? [...wideCapsules, ...landscapes] : landscapes;
			hasMoreWide = (grids || []).length >= 50;
			const animCount = landscapes.filter((p: any) => isAnimatedImage(p.mime, p.url)).length;
			statusMessage = `Loading ${landscapes.length} wide capsule images...`;
			widePage++;

			await preloadImages(landscapes);
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
			const data = await GetHeroes(selectedGameID, getCurrentFilters(), heroPage);
			const items = data || [];
			heroes = append ? [...heroes, ...items] : items;
			hasMoreHeroes = items.length >= 50;
			const animCount = items.filter((p: any) => isAnimatedImage(p.mime, p.url)).length;
			statusMessage = `Loading ${items.length} hero images...`;
			heroPage++;

			await preloadImages(items);
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
			const data = await GetLogos(selectedGameID, getCurrentFilters(), logoPage);
			const items = data || [];
			logos = append ? [...logos, ...items] : items;
			hasMoreLogos = items.length >= 50;
			statusMessage = `Loading ${items.length} logo images...`;
			logoPage++;

			await preloadImages(items);
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
			const data = await GetIcons(selectedGameID, getCurrentFilters(), iconPage);
			const items = data || [];
			icons = append ? [...icons, ...items] : items;
			hasMoreIcons = items.length >= 50;
			statusMessage = `Loading ${items.length} icon images...`;
			iconPage++;

			await preloadImages(items);
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
		showPreview(img.url, img.width, img.height, img.style, img.mime);
	}

	function selectWide(img: GridData) {
		gridLandscape = img.url;
		showPreview(img.url, img.width, img.height, img.style, img.mime);
	}

	function selectHero(img: ImageData) {
		heroImage = img.url;
		showPreview(img.url, img.width, img.height, img.style, img.mime);
	}

	function selectLogo(img: ImageData) {
		logoImage = img.url;
		showPreview(img.url, img.width, img.height, img.style, img.mime);
	}

	function selectIcon(img: ImageData) {
		iconImage = img.url;
		showPreview(img.url, img.width, img.height, img.style, img.mime);
	}

	function showPreview(url: string, width: number, height: number, style: string, mime: string) {
		// Use cached version for display if available
		previewUrl = imageCache.get(url) || url;
		const isAnim = isAnimatedImage(mime, url);
		previewInfo = `${width}x${height} - ${style}${isAnim ? ' (Animated)' : ''}`;
	}

	// Get cached image URL for display (returns original URL if not cached)
	function getCachedUrl(originalUrl: string): string {
		if (!originalUrl) return '';
		return imageCache.get(originalUrl) || originalUrl;
	}

	function getImageSrc(img: any): string {
		const url = img?.url || img?.Url || img?.URL || '';
		if (!url) return '';

		// Return cached data URL if available, otherwise return original URL
		const cached = imageCache.get(url);
		return cached || url;
	}

	// Preload images through proxy (runs in background, images show immediately with original URL)
	async function preloadImages(images: any[]) {
		console.log('[preloadImages] Starting with', images.length, 'images');
		const urls = images.map(img => img?.url || img?.Url || img?.URL || '').filter(Boolean);

		const uncachedUrls = urls.filter(url => !imageCache.has(url) && !loadingImages.has(url));
		console.log('[preloadImages] Uncached URLs:', uncachedUrls.length);

		if (uncachedUrls.length === 0) {
			console.log('[preloadImages] All images already cached');
			return;
		}

		// Mark as loading
		uncachedUrls.forEach(url => loadingImages.add(url));
		loadingImages = new Set(loadingImages);

		// Load in parallel (batch of 3)
		const batchSize = 3;
		for (let i = 0; i < uncachedUrls.length; i += batchSize) {
			const batch = uncachedUrls.slice(i, i + batchSize);
			console.log('[preloadImages] Loading batch', Math.floor(i / batchSize) + 1);

			await Promise.all(batch.map(async (url) => {
				try {
					console.log('[preloadImages] Proxying:', url.substring(0, 60) + '...');
					const dataUrl = await ProxyImage(url);
					console.log('[preloadImages] Got data URL, length:', dataUrl?.length || 0);
					if (dataUrl && dataUrl.startsWith('data:')) {
						imageCache.set(url, dataUrl);
					}
				} catch (err) {
					console.error('[preloadImages] Failed:', err);
				} finally {
					loadingImages.delete(url);
				}
			}));
			// Trigger reactivity after each batch
			imageCache = new Map(imageCache);
			loadingImages = new Set(loadingImages);
		}
		console.log('[preloadImages] Done, cache size:', imageCache.size);
	}

	// Handle image load error - try to load full URL if thumb fails
	function handleImageError(event: Event, img: { url?: string; thumb?: string }) {
		const target = event.target as HTMLImageElement;
		const thumb = img.thumb || '';
		const url = img.url || '';

		console.warn('Image load error:', { currentSrc: target.src, thumb, url });

		// If current src is thumb, try full URL
		if (target.src === thumb && url && url !== thumb) {
			console.log('Trying full URL:', url);
			target.src = url;
		} else {
			// Show placeholder - gray box with icon
			target.style.visibility = 'hidden';
			target.classList.add('load-failed');
		}
	}

	function clearAll() {
		gridPortrait = '';
		gridLandscape = '';
		heroImage = '';
		logoImage = '';
		iconImage = '';
		previewUrl = '';
		previewInfo = '';
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

	function openInBrowser() {
		if (previewUrl) {
			window.open(previewUrl, '_blank');
		}
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
		if (gameName && !currentSelection?.gridDBGameID) {
			searchGames();
		}
	});
</script>

<!-- Full screen overlay dialog -->
<div class="fixed inset-0 z-50 bg-background flex flex-col h-screen">
	<!-- Header -->
	<div class="flex items-center justify-between p-3 border-b shrink-0">
		<h2 class="text-lg font-semibold">Select Artwork - {gameName}</h2>
		<Button variant="ghost" size="icon" onclick={onclose}>
			<X class="w-5 h-5" />
		</Button>
	</div>

	<!-- Main content -->
	<div class="flex-1 flex min-h-0">
		<!-- Left panel: Search -->
		<div class="w-56 border-r flex flex-col shrink-0">
			<div class="p-3 space-y-2 shrink-0">
				<h3 class="font-semibold text-sm">Search SteamGridDB</h3>
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
						onclick={() => activeTab = tab.id}
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
				<Button variant="ghost" size="sm" onclick={() => showFilters = !showFilters}>
					<Filter class="w-4 h-4 mr-1" />
					Filters
				</Button>
				<Button variant="ghost" size="sm" onclick={reloadCurrentTab} disabled={loading || !selectedGameID}>
					<RefreshCw class={cn('w-4 h-4', loading && 'animate-spin')} />
				</Button>
			</div>

			<!-- Filters panel -->
			{#if showFilters}
				<div class="p-2 border-b bg-muted/50 shrink-0">
					<div class="flex flex-wrap items-center gap-3">
						<div class="flex items-center gap-1">
							<span class="text-xs text-muted-foreground w-12">Style:</span>
							<Select
								options={getStyleOptions()}
								value={filterStyle}
								onchange={(v) => filterStyle = v}
								placeholder="All"
								class="w-28"
							/>
						</div>
						<div class="flex items-center gap-1">
							<span class="text-xs text-muted-foreground w-14">Format:</span>
							<Select
								options={getMimeOptions()}
								value={filterMime}
								onchange={(v) => filterMime = v}
								placeholder="All"
								class="w-32"
							/>
						</div>
						<div class="flex items-center gap-1">
							<span class="text-xs text-muted-foreground w-10">Size:</span>
							<Select
								options={getDimensionOptions()}
								value={filterDimension}
								onchange={(v) => filterDimension = v}
								placeholder="All"
								class="w-28"
							/>
						</div>
						<div class="flex items-center gap-1">
							<span class="text-xs text-muted-foreground w-16">Animation:</span>
							<Select
								options={animationOptions}
								value={filterAnimation}
								onchange={(v) => filterAnimation = v}
								placeholder="All"
								class="w-32"
							/>
						</div>
						<Checkbox
							checked={filterNsfw}
							onchange={(v) => filterNsfw = v}
							label="NSFW"
						/>
						<Checkbox
							checked={filterHumor}
							onchange={(v) => filterHumor = v}
							label="Humor"
						/>
						<Button variant="outline" size="sm" onclick={reloadCurrentTab} disabled={loading}>
							Apply
						</Button>
					</div>
				</div>
			{/if}

			<!-- Image grid -->
			<div class="flex-1 overflow-y-auto p-2 min-h-0">
				{#if activeTab === 'capsule'}
					<div class="text-xs text-muted-foreground mb-2">600x900 - Portrait capsule</div>
					<div class="grid grid-cols-5 gap-2">
						{#each capsules as img}
							{@const isAnim = isAnimatedImage(img.mime, img.url)}
							{@const selected = isSelected(img.url, 'capsule')}
							<button
								type="button"
								class={cn(
									'relative rounded-lg overflow-hidden border-2 transition-all',
									selected ? 'border-green-500 ring-2 ring-green-500/50' : 'border-transparent hover:border-blue-500'
								)}
								onclick={() => selectCapsule(img)}
							>
								<img
									src={getImageSrc(img)}
									alt=""
									class="w-full aspect-[2/3] object-cover bg-muted"
									loading="lazy"
									onerror={(e) => handleImageError(e, img)}
								/>
								{#if selected}
									<div class="absolute top-1 right-1 bg-green-500 rounded-full p-0.5">
										<Check class="w-3 h-3 text-white" />
									</div>
								{/if}
								{#if isAnim}
									<span class="absolute top-1 left-1 bg-orange-500 text-white text-[9px] px-1 rounded font-bold">ANIM</span>
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
					<div class="grid grid-cols-3 gap-2">
						{#each wideCapsules as img}
							{@const isAnim = isAnimatedImage(img.mime, img.url)}
							{@const selected = isSelected(img.url, 'wide')}
							<button
								type="button"
								class={cn(
									'relative rounded-lg overflow-hidden border-2 transition-all',
									selected ? 'border-green-500 ring-2 ring-green-500/50' : 'border-transparent hover:border-blue-500'
								)}
								onclick={() => selectWide(img)}
							>
								<img
									src={getImageSrc(img)}
									alt=""
									class="w-full aspect-[460/215] object-cover bg-muted"
									loading="lazy"
									onerror={(e) => handleImageError(e, img)}
								/>
								{#if selected}
									<div class="absolute top-1 right-1 bg-green-500 rounded-full p-0.5">
										<Check class="w-3 h-3 text-white" />
									</div>
								{/if}
								{#if isAnim}
									<span class="absolute top-1 left-1 bg-orange-500 text-white text-[9px] px-1 rounded font-bold">ANIM</span>
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
					<div class="grid grid-cols-2 gap-2">
						{#each heroes as img}
							{@const isAnim = isAnimatedImage(img.mime, img.url)}
							{@const selected = isSelected(img.url, 'hero')}
							<button
								type="button"
								class={cn(
									'relative rounded-lg overflow-hidden border-2 transition-all',
									selected ? 'border-green-500 ring-2 ring-green-500/50' : 'border-transparent hover:border-blue-500'
								)}
								onclick={() => selectHero(img)}
							>
								<img
									src={getImageSrc(img)}
									alt=""
									class="w-full aspect-[1920/620] object-cover bg-muted"
									loading="lazy"
									onerror={(e) => handleImageError(e, img)}
								/>
								{#if selected}
									<div class="absolute top-1 right-1 bg-green-500 rounded-full p-0.5">
										<Check class="w-3 h-3 text-white" />
									</div>
								{/if}
								{#if isAnim}
									<span class="absolute top-1 left-1 bg-orange-500 text-white text-[9px] px-1 rounded font-bold">ANIM</span>
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
					<div class="grid grid-cols-5 gap-2">
						{#each logos as img}
							{@const selected = isSelected(img.url, 'logo')}
							<button
								type="button"
								class={cn(
									'relative rounded-lg overflow-hidden border-2 transition-all bg-muted p-1',
									selected ? 'border-green-500 ring-2 ring-green-500/50' : 'border-transparent hover:border-blue-500'
								)}
								onclick={() => selectLogo(img)}
							>
								<img
									src={getImageSrc(img)}
									alt=""
									class="w-full aspect-square object-contain"
									loading="lazy"
									onerror={(e) => handleImageError(e, img)}
								/>
								{#if selected}
									<div class="absolute top-1 right-1 bg-green-500 rounded-full p-0.5">
										<Check class="w-3 h-3 text-white" />
									</div>
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
					<div class="grid grid-cols-8 gap-2">
						{#each icons as img}
							{@const selected = isSelected(img.url, 'icon')}
							<button
								type="button"
								class={cn(
									'relative rounded-lg overflow-hidden border-2 transition-all bg-muted p-0.5',
									selected ? 'border-green-500 ring-2 ring-green-500/50' : 'border-transparent hover:border-blue-500'
								)}
								onclick={() => selectIcon(img)}
							>
								<img
									src={getImageSrc(img)}
									alt=""
									class="w-full aspect-square object-contain"
									loading="lazy"
									onerror={(e) => handleImageError(e, img)}
								/>
								{#if selected}
									<div class="absolute top-0.5 right-0.5 bg-green-500 rounded-full p-0.5">
										<Check class="w-2 h-2 text-white" />
									</div>
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

		<!-- Right panel: Preview & Selection -->
		<div class="w-64 border-l flex flex-col shrink-0">
			<div class="p-3 border-b shrink-0">
				<h3 class="font-semibold text-sm mb-2">Preview</h3>
				{#if previewUrl}
					<img src={previewUrl} alt="Preview" class="w-full max-h-40 object-contain rounded-lg bg-muted" />
					<p class="text-xs text-muted-foreground mt-1 text-center">{previewInfo}</p>
					<Button variant="outline" size="sm" class="w-full mt-2" onclick={openInBrowser}>
						<ExternalLink class="w-3 h-3 mr-1" />
						Open Full Size
					</Button>
				{:else}
					<div class="h-32 flex items-center justify-center bg-muted rounded-lg">
						<p class="text-xs text-muted-foreground">Select an image</p>
					</div>
				{/if}
			</div>

			<!-- Current selections with thumbnails -->
			<div class="flex-1 overflow-y-auto p-3 min-h-0">
				<h3 class="font-semibold text-sm mb-2">Selected Artwork</h3>
				<div class="space-y-3 text-xs">
					<!-- Capsule -->
					<div class="flex items-center gap-2">
						<span class="w-14 text-muted-foreground shrink-0">Capsule:</span>
						{#if gridPortrait && gridPortrait.length > 0}
							<div class="flex items-center gap-2 flex-1 min-w-0">
								<img src={getCachedUrl(gridPortrait)} alt="Capsule" class="h-10 w-auto rounded border border-green-500 object-contain" />
								<Check class="w-3 h-3 text-green-500 shrink-0" />
							</div>
						{:else}
							<span class="text-muted-foreground italic">None</span>
						{/if}
					</div>
					<!-- Wide -->
					<div class="flex items-center gap-2">
						<span class="w-14 text-muted-foreground shrink-0">Wide:</span>
						{#if gridLandscape && gridLandscape.length > 0}
							<div class="flex items-center gap-2 flex-1 min-w-0">
								<img src={getCachedUrl(gridLandscape)} alt="Wide" class="h-8 w-auto rounded border border-green-500 object-contain" />
								<Check class="w-3 h-3 text-green-500 shrink-0" />
							</div>
						{:else}
							<span class="text-muted-foreground italic">None</span>
						{/if}
					</div>
					<!-- Hero -->
					<div class="flex items-center gap-2">
						<span class="w-14 text-muted-foreground shrink-0">Hero:</span>
						{#if heroImage && heroImage.length > 0}
							<div class="flex items-center gap-2 flex-1 min-w-0">
								<img src={getCachedUrl(heroImage)} alt="Hero" class="h-6 w-auto rounded border border-green-500 object-contain" />
								<Check class="w-3 h-3 text-green-500 shrink-0" />
							</div>
						{:else}
							<span class="text-muted-foreground italic">None</span>
						{/if}
					</div>
					<!-- Logo -->
					<div class="flex items-center gap-2">
						<span class="w-14 text-muted-foreground shrink-0">Logo:</span>
						{#if logoImage && logoImage.length > 0}
							<div class="flex items-center gap-2 flex-1 min-w-0">
								<img src={getCachedUrl(logoImage)} alt="Logo" class="h-8 w-auto rounded border border-green-500 object-contain bg-muted" />
								<Check class="w-3 h-3 text-green-500 shrink-0" />
							</div>
						{:else}
							<span class="text-muted-foreground italic">None</span>
						{/if}
					</div>
					<!-- Icon -->
					<div class="flex items-center gap-2">
						<span class="w-14 text-muted-foreground shrink-0">Icon:</span>
						{#if iconImage && iconImage.length > 0}
							<div class="flex items-center gap-2 flex-1 min-w-0">
								<img src={getCachedUrl(iconImage)} alt="Icon" class="h-8 w-8 rounded border border-green-500 object-contain bg-muted" />
								<Check class="w-3 h-3 text-green-500 shrink-0" />
							</div>
						{:else}
							<span class="text-muted-foreground italic">None</span>
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
