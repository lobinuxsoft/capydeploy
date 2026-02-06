<script lang="ts">
	import { Dialog, Button, Toggle, MultiSelect } from '$lib/components/ui';
	import type { ImageFilters } from '$lib/types';
	import {
		gridStyles, heroStyles, logoStyles, iconStyles,
		capsuleDimensions, wideCapsuleDimensions, heroDimensions, logoDimensions, iconDimensions,
		gridMimes, logoMimes, iconMimes
	} from '$lib/types';
	import { RotateCcw } from 'lucide-svelte';

	type AssetType = 'capsule' | 'wide' | 'hero' | 'logo' | 'icon';

	interface Props {
		open?: boolean;
		assetType: AssetType;
		filters: ImageFilters;
		supportedFormats?: string[];
		onclose?: () => void;
		onapply?: (filters: ImageFilters) => void;
	}

	let {
		open = $bindable(false),
		assetType,
		filters,
		supportedFormats = [],
		onclose,
		onapply
	}: Props = $props();

	// Local state for editing
	let selectedStyles = $state<string[]>([]);
	let selectedMimes = $state<string[]>([]);
	let selectedDimensions = $state<string[]>([]);
	let showAnimated = $state(true);
	let showStatic = $state(true);
	let showNsfw = $state(false);
	let showHumor = $state(true);

	// Initialize from props when modal opens
	$effect(() => {
		if (open) {
			// Parse existing filters (comma-separated strings to arrays)
			selectedStyles = filters.style ? filters.style.split(',').filter(Boolean) : [];
			selectedMimes = filters.mimeType ? filters.mimeType.split(',').filter(Boolean) : [];
			selectedDimensions = filters.dimension ? filters.dimension.split(',').filter(Boolean) : [];

			// Parse imageType
			if (filters.imageType === 'Static Only') {
				showAnimated = false;
				showStatic = true;
			} else if (filters.imageType === 'Animated Only') {
				showAnimated = true;
				showStatic = false;
			} else {
				showAnimated = true;
				showStatic = true;
			}

			showNsfw = filters.showNsfw;
			showHumor = filters.showHumor;
		}
	});

	// Get options based on asset type (following decky-steamgriddb defaults)
	function getStyleOptions() {
		const styles = (() => {
			switch (assetType) {
				case 'capsule':
				case 'wide': return gridStyles;
				case 'hero': return heroStyles;
				case 'logo': return logoStyles;
				case 'icon': return iconStyles;
				default: return gridStyles;
			}
		})();
		return styles.filter(s => s !== 'All Styles').map(s => ({ value: s, label: formatLabel(s) }));
	}

	function getDimensionOptions() {
		const dims = (() => {
			switch (assetType) {
				case 'capsule': return capsuleDimensions;
				case 'wide': return wideCapsuleDimensions;
				case 'hero': return heroDimensions;
				case 'logo': return logoDimensions;
				case 'icon': return iconDimensions;
				default: return capsuleDimensions;
			}
		})();
		return dims.filter(d => d !== 'All Sizes').map(d => ({ value: d, label: d.replace('x', '×') }));
	}

	function getMimeOptions() {
		let mimes = (() => {
			switch (assetType) {
				case 'logo': return logoMimes;
				case 'icon': return iconMimes;
				default: return gridMimes;
			}
		})();

		// Filter by supported formats if available
		if (supportedFormats.length > 0) {
			mimes = mimes.filter(m => m === 'All Formats' || supportedFormats.includes(m));
		}

		return mimes.filter(m => m !== 'All Formats').map(m => ({
			value: m,
			label: m.replace('image/', '').replace('vnd.microsoft.', '').toUpperCase()
		}));
	}

	function formatLabel(s: string): string {
		return s.replace(/_/g, ' ').replace(/\b\w/g, c => c.toUpperCase());
	}

	// Check if filters differ from defaults
	function hasChanges(): boolean {
		return selectedStyles.length > 0 ||
			selectedMimes.length > 0 ||
			selectedDimensions.length > 0 ||
			!showAnimated ||
			!showStatic ||
			showNsfw ||
			!showHumor;
	}

	function resetFilters() {
		selectedStyles = [];
		selectedMimes = [];
		selectedDimensions = [];
		showAnimated = true;
		showStatic = true;
		showNsfw = false;
		showHumor = true;
	}

	function handleApply() {
		// Build ImageType filter
		let imageType = '';
		if (showAnimated && !showStatic) imageType = 'Animated Only';
		else if (showStatic && !showAnimated) imageType = 'Static Only';

		const newFilters: ImageFilters = {
			style: selectedStyles.join(','),
			mimeType: selectedMimes.join(','),
			dimension: selectedDimensions.join(','),
			imageType,
			showNsfw,
			showHumor
		};

		onapply?.(newFilters);
		open = false;
	}

	function handleClose() {
		open = false;
		onclose?.();
	}
</script>

<Dialog bind:open title="Filters" showMascot={false} onclose={handleClose} class="max-w-md">
	<div class="space-y-4">
		<!-- Styles -->
		<div class="space-y-1.5">
			<label class="text-sm font-medium text-muted-foreground">Styles</label>
			<MultiSelect
				options={getStyleOptions()}
				bind:selected={selectedStyles}
				placeholder="All styles"
				class="w-full"
			/>
		</div>

		<!-- Dimensions -->
		{#if getDimensionOptions().length > 0}
			<div class="space-y-1.5">
				<label class="text-sm font-medium text-muted-foreground">Dimensions</label>
				<MultiSelect
					options={getDimensionOptions()}
					bind:selected={selectedDimensions}
					placeholder="All sizes"
					class="w-full"
				/>
			</div>
		{/if}

		<!-- File Types -->
		<div class="space-y-1.5">
			<label class="text-sm font-medium text-muted-foreground">File Types</label>
			<MultiSelect
				options={getMimeOptions()}
				bind:selected={selectedMimes}
				placeholder="All formats"
				class="w-full"
			/>
		</div>

		<!-- Types Section -->
		<div class="space-y-2">
			<label class="text-sm font-medium text-muted-foreground">Types</label>
			<div class="flex gap-6">
				<Toggle bind:checked={showAnimated} label="Animated" />
				<Toggle bind:checked={showStatic} label="Static" />
			</div>
			{#if !showAnimated && !showStatic}
				<p class="text-xs text-destructive">At least one type must be selected</p>
			{/if}
		</div>

		<!-- Tags Section -->
		<div class="space-y-2">
			<label class="text-sm font-medium text-muted-foreground">Tags</label>
			<div class="flex flex-wrap gap-4">
				<Toggle bind:checked={showNsfw} label="Adult Content" />
				<Toggle bind:checked={showHumor} label="Humor" />
			</div>
		</div>

		<!-- Actions -->
		<div class="flex items-center justify-between pt-2 border-t border-border">
			{#if hasChanges()}
				<Button variant="ghost" size="sm" onclick={resetFilters}>
					<RotateCcw class="w-4 h-4 mr-1" />
					Reset
				</Button>
			{:else}
				<div></div>
			{/if}
			<div class="flex gap-2">
				<Button variant="outline" size="sm" onclick={handleClose}>
					Cancel
				</Button>
				<Button size="sm" onclick={handleApply} disabled={!showAnimated && !showStatic}>
					Apply
				</Button>
			</div>
		</div>
	</div>
</Dialog>
