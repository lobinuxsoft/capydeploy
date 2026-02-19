import type { GridData, ImageData, ImageFilters } from '$lib/types';

// --- Types ---

export type ArtworkTabId = 'capsule' | 'wide' | 'hero' | 'logo' | 'icon';

export interface ArtworkTabConfig {
	id: ArtworkTabId;
	label: string;
	description: string;
	cols: number;
	aspect: string;
	objectFit: 'cover' | 'contain';
	buttonClass?: string;
	compact?: boolean;
	dimensionStyle: 'overlay' | 'text';
	showHeight: boolean;
	preview: { imgClass: string; placeholderClass: string };
}

export interface ArtworkTabState {
	readonly items: (GridData | ImageData)[];
	readonly hasMore: boolean;
	selectedUrl: string;
	load(gameID: number, filters: ImageFilters, append: boolean): Promise<string>;
	reset(): void;
}

export type ArtworkLoader = (
	gameID: number,
	filters: ImageFilters,
	page: number
) => Promise<(GridData | ImageData)[] | null>;

// --- Config ---

export const TAB_CONFIGS: ArtworkTabConfig[] = [
	{
		id: 'capsule', label: 'Capsule', description: '600x900 - Portrait capsule',
		cols: 4, aspect: 'aspect-[2/3]', objectFit: 'cover',
		dimensionStyle: 'overlay', showHeight: true,
		preview: { imgClass: 'h-14 w-auto rounded border-2 border-green-500 object-contain', placeholderClass: 'h-14 w-10 rounded border border-dashed border-muted-foreground/30 flex items-center justify-center' }
	},
	{
		id: 'wide', label: 'Wide', description: '920x430 - Wide capsule',
		cols: 2, aspect: 'aspect-[460/215]', objectFit: 'cover',
		dimensionStyle: 'overlay', showHeight: true,
		preview: { imgClass: 'h-10 w-auto rounded border-2 border-green-500 object-contain', placeholderClass: 'h-10 w-20 rounded border border-dashed border-muted-foreground/30 flex items-center justify-center' }
	},
	{
		id: 'hero', label: 'Hero', description: '1920x620 - Hero banner',
		cols: 2, aspect: 'aspect-[1920/620]', objectFit: 'cover',
		dimensionStyle: 'overlay', showHeight: true,
		preview: { imgClass: 'h-8 w-auto rounded border-2 border-green-500 object-contain', placeholderClass: 'h-8 w-24 rounded border border-dashed border-muted-foreground/30 flex items-center justify-center' }
	},
	{
		id: 'logo', label: 'Logo', description: 'Game logo (transparent)',
		cols: 4, aspect: 'aspect-square', objectFit: 'contain',
		buttonClass: 'bg-muted p-1', dimensionStyle: 'text', showHeight: true,
		preview: { imgClass: 'h-10 w-auto rounded border-2 border-green-500 object-contain bg-muted/50', placeholderClass: 'h-10 w-16 rounded border border-dashed border-muted-foreground/30 flex items-center justify-center' }
	},
	{
		id: 'icon', label: 'Icon', description: 'Square icon',
		cols: 6, aspect: 'aspect-square', objectFit: 'contain',
		buttonClass: 'bg-muted p-0.5', compact: true, dimensionStyle: 'text', showHeight: false,
		preview: { imgClass: 'h-10 w-10 rounded border-2 border-green-500 object-contain bg-muted/50', placeholderClass: 'h-10 w-10 rounded border border-dashed border-muted-foreground/30 flex items-center justify-center' }
	}
];

// --- Helpers ---

export function isAnimatedThumb(thumb: string): boolean {
	return thumb?.includes('.webm') || false;
}

// --- Composable ---

export function createArtworkTab(
	config: ArtworkTabConfig,
	loader: ArtworkLoader,
	itemFilter?: (items: (GridData | ImageData)[]) => (GridData | ImageData)[]
): ArtworkTabState {
	let items = $state<(GridData | ImageData)[]>([]);
	let hasMore = $state(false);
	let selectedUrl = $state('');
	let page = $state(0);

	async function load(gameID: number, filters: ImageFilters, append: boolean): Promise<string> {
		if (!gameID) return '';
		if (!append) {
			page = 0;
			items = [];
		}
		const raw = await loader(gameID, filters, page);
		const loaded = raw || [];
		const filtered = itemFilter ? itemFilter(loaded) : loaded;
		items = append ? [...items, ...filtered] : filtered;
		hasMore = loaded.length >= 50;
		const animCount = filtered.filter((p) => isAnimatedThumb(p.thumb)).length;
		page++;
		return `Loaded ${filtered.length} ${config.label.toLowerCase()}s${animCount ? ` (${animCount} animated)` : ''}`;
	}

	function reset() {
		items = [];
		hasMore = false;
		page = 0;
	}

	return {
		get items() { return items; },
		get hasMore() { return hasMore; },
		get selectedUrl() { return selectedUrl; },
		set selectedUrl(url: string) { selectedUrl = url; },
		load,
		reset
	};
}
