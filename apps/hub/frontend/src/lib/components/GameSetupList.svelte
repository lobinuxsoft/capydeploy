<script lang="ts">
	import { Button, Card, Dialog, Input, Progress } from '$lib/components/ui';
	import { gameSetups, uploadProgress } from '$lib/stores/games';
	import { connectionStatus } from '$lib/stores/connection';
	import type { GameSetup, UploadProgress, ArtworkSelection } from '$lib/types';
	import { truncatePath } from '$lib/utils';
	import { Folder, Upload, Pencil, Trash2, Plus, Image, Loader2 } from 'lucide-svelte';
	import ArtworkSelector from './ArtworkSelector.svelte';
	import {
		GetGameSetups, AddGameSetup, UpdateGameSetup, RemoveGameSetup,
		SelectFolder, UploadGame, EventsOn, EventsOff
	} from '$lib/wailsjs';
	import { browser } from '$app/environment';

	let showSetupForm = $state(false);
	let showArtworkSelector = $state(false);
	let editingSetup: GameSetup | null = $state(null);
	let uploading = $state<string | null>(null);

	// Form state
	let formName = $state('');
	let formLocalPath = $state('');
	let formExecutable = $state('');
	let formLaunchOptions = $state('');
	let formTags = $state('');
	let formArtwork = $state<ArtworkSelection | null>(null);

	async function loadSetups() {
		if (!browser) return;
		try {
			const list = await GetGameSetups();
			gameSetups.set(list || []);
		} catch (e) {
			console.error('Failed to load game setups:', e);
		}
	}

	$effect(() => {
		if (!browser) return;

		loadSetups();

		// Listen for upload progress events
		EventsOn('upload:progress', (data: UploadProgress) => {
			uploadProgress.set(data);
			if (data.done) {
				uploading = null;
				if (!data.error) {
					alert('Upload complete: ' + data.status);
				} else {
					alert('Upload failed: ' + data.error);
				}
			}
		});

		return () => {
			EventsOff('upload:progress');
		};
	});

	function resetForm() {
		formName = '';
		formLocalPath = '';
		formExecutable = '';
		formLaunchOptions = '';
		formTags = '';
		formArtwork = null;
		editingSetup = null;
	}

	function openAddForm() {
		resetForm();
		showSetupForm = true;
	}

	function openEditForm(setup: GameSetup) {
		editingSetup = setup;
		formName = setup.name;
		formLocalPath = setup.local_path;
		formExecutable = setup.executable;
		formLaunchOptions = setup.launch_options || '';
		formTags = setup.tags || '';
		if (setup.griddb_game_id || setup.grid_portrait || setup.grid_landscape ||
			setup.hero_image || setup.logo_image || setup.icon_image) {
			formArtwork = {
				gridDBGameID: setup.griddb_game_id || 0,
				gridPortrait: setup.grid_portrait || '',
				gridLandscape: setup.grid_landscape || '',
				heroImage: setup.hero_image || '',
				logoImage: setup.logo_image || '',
				iconImage: setup.icon_image || ''
			};
		}
		showSetupForm = true;
	}

	async function selectFolderHandler() {
		try {
			const folder = await SelectFolder();
			if (folder) {
				formLocalPath = folder;
				if (!formName) {
					// Extract folder name
					const parts = folder.split(/[/\\]/);
					formName = parts[parts.length - 1] || '';
				}
			}
		} catch (e) {
			console.error('Failed to select folder:', e);
		}
	}

	async function saveSetup() {
		if (!formName || !formLocalPath || !formExecutable) {
			alert('Name, Local Folder, and Executable are required');
			return;
		}

		const setup: GameSetup = {
			id: editingSetup?.id || '',
			name: formName,
			local_path: formLocalPath,
			executable: formExecutable,
			launch_options: formLaunchOptions,
			tags: formTags,
			install_path: '', // Agent decides the install path
			griddb_game_id: formArtwork?.gridDBGameID,
			grid_portrait: formArtwork?.gridPortrait,
			grid_landscape: formArtwork?.gridLandscape,
			hero_image: formArtwork?.heroImage,
			logo_image: formArtwork?.logoImage,
			icon_image: formArtwork?.iconImage
		};

		try {
			if (editingSetup) {
				await UpdateGameSetup(editingSetup.id, setup);
			} else {
				await AddGameSetup(setup);
			}
			await loadSetups();
			showSetupForm = false;
			resetForm();
		} catch (e) {
			console.error('Failed to save setup:', e);
			alert('Error: ' + e);
		}
	}

	async function deleteSetup(id: string, name: string) {
		if (!confirm(`Delete setup for '${name}'?`)) return;
		try {
			await RemoveGameSetup(id);
			await loadSetups();
		} catch (e) {
			console.error('Failed to delete setup:', e);
		}
	}

	async function uploadGameHandler(setup: GameSetup) {
		if (!$connectionStatus.connected) {
			alert('No device connected');
			return;
		}

		uploading = setup.id;
		uploadProgress.set({ progress: 0, status: 'Starting upload...', done: false });

		try {
			await UploadGame(setup.id);
		} catch (e) {
			console.error('Failed to start upload:', e);
			alert('Error: ' + e);
			uploading = null;
			uploadProgress.set(null);
		}
	}

	function countArtwork(setup: GameSetup): number {
		let count = 0;
		if (setup.grid_portrait) count++;
		if (setup.grid_landscape) count++;
		if (setup.hero_image) count++;
		if (setup.logo_image) count++;
		if (setup.icon_image) count++;
		return count;
	}

	function handleArtworkSave(selection: ArtworkSelection) {
		formArtwork = selection;
		showArtworkSelector = false;
	}
</script>

<div class="space-y-4">
	<div class="flex gap-2">
		<Button onclick={openAddForm}>
			<Plus class="w-4 h-4 mr-2" />
			New Game Setup
		</Button>
	</div>

	<p class="text-sm text-muted-foreground">
		Saved Game Setups (click upload icon to install):
	</p>

	<div class="space-y-2">
		{#each $gameSetups as setup}
			{@const artworkCount = countArtwork(setup)}
			{@const isUploading = uploading === setup.id}
			<Card class="p-4">
				<div class="flex items-center justify-between">
					<div class="flex items-center gap-3">
						<Folder class="w-6 h-6 text-muted-foreground" />
						<div>
							<div class="flex items-center gap-2">
								<span class="font-medium">{setup.name}</span>
								{#if artworkCount > 0}
									<span class="text-xs text-muted-foreground flex items-center gap-1">
										<Image class="w-3 h-3" />
										{artworkCount}
									</span>
								{/if}
							</div>
							<div class="text-sm text-muted-foreground">
								{truncatePath(setup.local_path, 40)}
							</div>
						</div>
					</div>
					<div class="flex gap-1">
						<Button
							size="icon"
							onclick={() => uploadGameHandler(setup)}
							disabled={isUploading || !$connectionStatus.connected}
						>
							{#if isUploading}
								<Loader2 class="w-4 h-4 animate-spin" />
							{:else}
								<Upload class="w-4 h-4" />
							{/if}
						</Button>
						<Button variant="ghost" size="icon" onclick={() => openEditForm(setup)}>
							<Pencil class="w-4 h-4" />
						</Button>
						<Button variant="ghost" size="icon" onclick={() => deleteSetup(setup.id, setup.name)}>
							<Trash2 class="w-4 h-4" />
						</Button>
					</div>
				</div>
			</Card>
		{/each}

		{#if $gameSetups.length === 0}
			<div class="text-center text-muted-foreground py-8">
				No game setups configured. Create a new setup to get started.
			</div>
		{/if}
	</div>

	<!-- Upload Progress -->
	{#if $uploadProgress && !$uploadProgress.done}
		<Card class="p-4 space-y-2">
			<div class="flex justify-between text-sm">
				<span>{$uploadProgress.status}</span>
				<span>{Math.round($uploadProgress.progress * 100)}%</span>
			</div>
			<Progress value={$uploadProgress.progress * 100} />
		</Card>
	{/if}
</div>

<!-- Game Setup Form Dialog -->
<Dialog bind:open={showSetupForm} title={editingSetup ? 'Edit Game Setup' : 'New Game Setup'} class="max-w-lg">
	<div class="space-y-4">
		<div class="space-y-2">
			<label class="text-sm font-medium">Game Name</label>
			<Input bind:value={formName} placeholder="My Game" />
		</div>

		<div class="space-y-2">
			<label class="text-sm font-medium">Local Folder</label>
			<div class="flex gap-2">
				<Input bind:value={formLocalPath} placeholder="Select folder..." class="flex-1" />
				<Button variant="outline" onclick={selectFolderHandler}>
					<Folder class="w-4 h-4" />
				</Button>
			</div>
		</div>

		<div class="space-y-2">
			<label class="text-sm font-medium">Executable</label>
			<Input bind:value={formExecutable} placeholder="game.x86_64 or game.sh" />
		</div>

		<div class="space-y-2">
			<label class="text-sm font-medium">Launch Options</label>
			<Input bind:value={formLaunchOptions} placeholder="Optional launch arguments" />
		</div>

		<div class="space-y-2">
			<label class="text-sm font-medium">Tags</label>
			<Input bind:value={formTags} placeholder="tag1, tag2 (optional)" />
		</div>

		<div class="space-y-2">
			<label class="text-sm font-medium">Artwork</label>
			<div class="flex items-center gap-2">
				<span class="text-sm text-muted-foreground">
					{#if formArtwork}
						{Object.values(formArtwork).filter(v => v && typeof v === 'string' && v.length > 0).length} artwork(s) selected
					{:else}
						No artwork selected
					{/if}
				</span>
				<Button variant="outline" size="sm" onclick={() => showArtworkSelector = true}>
					<Image class="w-4 h-4 mr-2" />
					Select Artwork
				</Button>
			</div>
		</div>

		<div class="flex justify-end gap-2 pt-4">
			<Button variant="outline" onclick={() => { showSetupForm = false; resetForm(); }}>
				Cancel
			</Button>
			<Button onclick={saveSetup}>
				Save Setup
			</Button>
		</div>
	</div>
</Dialog>

<!-- Artwork Selector -->
{#if showArtworkSelector}
	<ArtworkSelector
		gameName={formName || 'Game'}
		currentSelection={formArtwork}
		onsave={handleArtworkSave}
		onclose={() => showArtworkSelector = false}
	/>
{/if}
