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
	import { SearchGames, GetGrids, GetHeroes, GetLogos, GetIcons, GetCacheURL, GetStaticThumbnail, OpenCachedImage } from '$lib/wailsjs';
	import { BrowserOpenURL } from '$wailsjs/runtime/runtime';
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
	let previewOriginalUrl = $state(''); // Original URL for opening cached file
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

	// Cache URL mapping - maps original URLs to local cache URLs
	// These are lightweight strings, not image data
	let cacheURLs = $state<Map<string, string>>(new Map());
	let loadingPreview = $state(false);

	// Static thumbnail cache for animated images (prevents memory bloat from decoded GIF frames)
	let staticThumbnails = $state<Map<string, string>>(new Map());
	let loadingThumbnails = $state<Set<string>>(new Set());

	// Load static thumbnail for an animated image
	async function loadStaticThumbnail(url: string, mime: string) {
		if (!selectedGameID || !isAnimatedImage(mime, url)) return;
		if (staticThumbnails.has(url) || loadingThumbnails.has(url)) return;

		loadingThumbnails.add(url);
		try {
			const thumbUrl = await GetStaticThumbnail(selectedGameID, url, 200);
			if (thumbUrl) {
				staticThumbnails.set(url, thumbUrl);
				// Force reactivity update
				staticThumbnails = new Map(staticThumbnails);
			}
		} catch (e) {
			console.warn('Failed to load static thumbnail:', e);
		} finally {
			loadingThumbnails.delete(url);
		}
	}

	// Cleanup function to clear all cached data
	function clearCache() {
		cacheURLs.clear();
		staticThumbnails.clear();
		loadingThumbnails.clear();
		capsules = [];
		wideCapsules = [];
		heroes = [];
		logos = [];
		icons = [];
		console.log('[ArtworkSelector] Cache cleared');
	}

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

	// Filter MIMEs based on what the connected agent supports
	function filterMimes(mimes: string[], supported: string[]): string[] {
		if (!supported || supported.length === 0) return mimes;
		return mimes.filter(m => m === 'All Formats' || supported.includes(m));
	}

	// Reactive MIME options filtered by agent's supported formats
	let mimeOptions = $derived.by(() => {
		const supported = $connectionStatus.supportedImageFormats;
		switch (activeTab) {
			case 'logo': return filterMimes(logoMimes, supported);
			case 'icon': return filterMimes(iconMimes, supported);
			default: return filterMimes(gridMimes, supported);
		}
	});

	// Reset filterMime when it's no longer valid for current options
	$effect(() => {
		if (filterMime && !mimeOptions.includes(filterMime)) {
			filterMime = '';
		}
	});

	// Check if animated formats are supported (WebP/GIF can be animated)
	let supportsAnimated = $derived.by(() => {
		const supported = $connectionStatus.supportedImageFormats;
		if (!supported || supported.length === 0) return true; // No agent = show all
		return supported.includes('image/webp') || supported.includes('image/gif');
	});

	// Reset animation filter if animated not supported
	$effect(() => {
		if (!supportsAnimated && filterAnimation === 'Animated Only') {
			filterAnimation = '';
		}
	});

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
			const grids = await GetGrids(selectedGameID, getCurrentFilters(), widePage);
			const landscapes = (grids || []).filter((g: any) => g.width > g.height);
			wideCapsules = append ? [...wideCapsules, ...landscapes] : landscapes;
			hasMoreWide = (grids || []).length >= 50;
			const animCount = landscapes.filter((p: any) => isAnimatedImage(p.mime, p.url)).length;
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
			const data = await GetHeroes(selectedGameID, getCurrentFilters(), heroPage);
			const items = data || [];
			heroes = append ? [...heroes, ...items] : items;
			hasMoreHeroes = items.length >= 50;
			const animCount = items.filter((p: any) => isAnimatedImage(p.mime, p.url)).length;
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
			const data = await GetLogos(selectedGameID, getCurrentFilters(), logoPage);
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
			const data = await GetIcons(selectedGameID, getCurrentFilters(), iconPage);
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

	async function showPreview(url: string, width: number, height: number, style: string, mime: string) {
		// Save original URL for opening cached file
		previewOriginalUrl = url;
		const isAnim = isAnimatedImage(mime, url);
		previewInfo = `${width}x${height} - ${style}${isAnim ? ' (Animated)' : ''}`;

		// Check if we already have the cache URL
		const cached = cacheURLs.get(url);
		if (cached) {
			previewUrl = cached;
			return;
		}

		// Show original URL while loading (thumbnail or external)
		previewUrl = url;

		if (selectedGameID > 0) {
			loadingPreview = true;
			try {
				// GetCacheURL downloads the image to disk and returns a local URL
				// This avoids base64 encoding and keeps images out of JS memory
				const cacheUrl = await GetCacheURL(selectedGameID, url);
				if (cacheUrl) {
					cacheURLs.set(url, cacheUrl);
					previewUrl = cacheUrl;
				}
			} catch (e) {
				console.warn('Failed to get cache URL:', e);
				// Keep using the original URL as fallback
			} finally {
				loadingPreview = false;
			}
		}
	}

	// Get cached preview URL for selected artwork display
	function getCachedUrl(originalUrl: string): string {
		if (!originalUrl) return '';
		return cacheURLs.get(originalUrl) || originalUrl;
	}

	// Tiny transparent placeholder (1x1 pixel)
	const PLACEHOLDER = 'data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7';

	// Get the best image source for grid display
	// For animated images, use static thumbnail to save memory (NO animated thumbs!)
	function getImageSrc(img: any): string {
		const url = img?.url || img?.Url || img?.URL || '';
		const mime = img?.mime || '';

		// For animated images, ONLY use static thumbnail - never show animation in grid
		if (isAnimatedImage(mime, url)) {
			const staticThumb = staticThumbnails.get(url);
			if (staticThumb) return staticThumb;

			// Start loading and show placeholder until ready
			loadStaticThumbnail(url, mime);
			return PLACEHOLDER;
		}

		// For static images, use SteamGridDB thumbnail
		const thumb = img?.thumb || img?.Thumb || '';
		if (thumb) return thumb;

		return url || '';
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

	async function openInBrowser() {
		if (previewOriginalUrl && selectedGameID > 0) {
			try {
				// Open cached image with system's default image viewer
				await OpenCachedImage(selectedGameID, previewOriginalUrl);
			} catch (e) {
				// Fallback to opening URL in browser if not cached
				console.warn('Image not cached, opening URL:', e);
				BrowserOpenURL(previewOriginalUrl);
			}
		} else if (previewOriginalUrl) {
			BrowserOpenURL(previewOriginalUrl);
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
								options={mimeOptions}
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
						{#if supportsAnimated}
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
						{/if}
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
						{#each capsules as img (img.url)}
							{@const isAnim = isAnimatedImage(img.mime, img.url)}
							{@const selected = isSelected(img.url, 'capsule')}
							{@const thumbSrc = isAnim ? (staticThumbnails.get(img.url) || (loadStaticThumbnail(img.url, img.mime), PLACEHOLDER)) : (img.thumb || img.url)}
							<button
								type="button"
								class={cn(
									'relative rounded-lg overflow-hidden border-2 transition-all',
									selected ? 'border-green-500 ring-2 ring-green-500/50' : 'border-transparent hover:border-blue-500'
								)}
								onclick={() => selectCapsule(img)}
							>
								<img
									src={thumbSrc}
									alt=""
									class="w-full aspect-[2/3] object-cover bg-muted"
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
						{#each wideCapsules as img (img.url)}
							{@const isAnim = isAnimatedImage(img.mime, img.url)}
							{@const selected = isSelected(img.url, 'wide')}
							{@const thumbSrc = isAnim ? (staticThumbnails.get(img.url) || (loadStaticThumbnail(img.url, img.mime), PLACEHOLDER)) : (img.thumb || img.url)}
							<button
								type="button"
								class={cn(
									'relative rounded-lg overflow-hidden border-2 transition-all',
									selected ? 'border-green-500 ring-2 ring-green-500/50' : 'border-transparent hover:border-blue-500'
								)}
								onclick={() => selectWide(img)}
							>
								<img
									src={thumbSrc}
									alt=""
									class="w-full aspect-[460/215] object-cover bg-muted"
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
						{#each heroes as img (img.url)}
							{@const isAnim = isAnimatedImage(img.mime, img.url)}
							{@const selected = isSelected(img.url, 'hero')}
							{@const thumbSrc = isAnim ? (staticThumbnails.get(img.url) || (loadStaticThumbnail(img.url, img.mime), PLACEHOLDER)) : (img.thumb || img.url)}
							<button
								type="button"
								class={cn(
									'relative rounded-lg overflow-hidden border-2 transition-all',
									selected ? 'border-green-500 ring-2 ring-green-500/50' : 'border-transparent hover:border-blue-500'
								)}
								onclick={() => selectHero(img)}
							>
								<img
									src={thumbSrc}
									alt=""
									class="w-full aspect-[1920/620] object-cover bg-muted"
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
						{#each logos as img (img.url)}
							{@const isAnim = isAnimatedImage(img.mime, img.url)}
							{@const selected = isSelected(img.url, 'logo')}
							{@const thumbSrc = isAnim ? (staticThumbnails.get(img.url) || (loadStaticThumbnail(img.url, img.mime), PLACEHOLDER)) : (img.thumb || img.url)}
							<button
								type="button"
								class={cn(
									'relative rounded-lg overflow-hidden border-2 transition-all bg-muted p-1',
									selected ? 'border-green-500 ring-2 ring-green-500/50' : 'border-transparent hover:border-blue-500'
								)}
								onclick={() => selectLogo(img)}
							>
								<img
									src={thumbSrc}
									alt=""
									class="w-full aspect-square object-contain"
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
						{#each icons as img (img.url)}
							{@const isAnim = isAnimatedImage(img.mime, img.url)}
							{@const selected = isSelected(img.url, 'icon')}
							{@const thumbSrc = isAnim ? (staticThumbnails.get(img.url) || (loadStaticThumbnail(img.url, img.mime), PLACEHOLDER)) : (img.thumb || img.url)}
							<button
								type="button"
								class={cn(
									'relative rounded-lg overflow-hidden border-2 transition-all bg-muted p-0.5',
									selected ? 'border-green-500 ring-2 ring-green-500/50' : 'border-transparent hover:border-blue-500'
								)}
								onclick={() => selectIcon(img)}
							>
								<img
									src={thumbSrc}
									alt=""
									class="w-full aspect-square object-contain"
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
				<h3 class="font-semibold text-sm mb-2 gradient-text">Preview</h3>
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
				<h3 class="font-semibold text-sm mb-2 gradient-text">Selected Artwork</h3>
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
