<script lang="ts">
	import { Button } from '$lib/components/ui';
	import { toast } from '$lib/stores/toast';
	import type { FsEntry, UploadProgress } from '$lib/types';
	import { FsList, FsMkdir, FsDelete, FsRename, FsDownloadPath, FsDownloadBatch, FsUpload, FsUploadLocal, FsCancelTransfer, EventsOn } from '$lib/wailsjs';
	import { getCurrentWindow } from '@tauri-apps/api/window';
	import { browser } from '$app/environment';
	import {
		Folder, File, ArrowUp, RefreshCw, Loader2, Eye, EyeOff, ChevronRight,
		Download, Upload, Trash2, FolderPlus, Pencil, CheckSquare, Square, FolderOpen, X
	} from 'lucide-svelte';

	let entries = $state<FsEntry[]>([]);
	let currentPath = $state('');
	let loading = $state(false);
	let showHidden = $state(false);
	let truncated = $state(false);
	let dragOver = $state(false);
	let transferProgress = $state<UploadProgress | null>(null);

	// Selection
	let selectedPaths = $state<Set<string>>(new Set());

	// Rename
	let renamingEntry = $state<FsEntry | null>(null);
	let renameValue = $state('');

	// New folder
	let creatingFolder = $state(false);
	let newFolderName = $state('');

	// Context menu
	let contextMenu = $state<{ x: number; y: number; entry: FsEntry } | null>(null);

	let selectedCount = $derived(selectedPaths.size);

	let breadcrumbs = $derived(() => {
		if (!currentPath) return [];
		const isWindows = currentPath.includes('\\');
		const sep = isWindows ? '\\' : '/';
		const parts = currentPath.split(sep).filter(Boolean);
		const segments: { name: string; path: string }[] = [];
		let accumulated = isWindows ? '' : '/';
		for (const part of parts) {
			accumulated += (isWindows && segments.length > 0 ? '\\' : (segments.length > 0 ? '/' : '')) + part;
			segments.push({ name: part, path: accumulated });
		}
		return segments;
	});

	// Transfer progress listener
	$effect(() => {
		if (!browser) return;

		const unsub = EventsOn('filebrowser:progress', (data: UploadProgress) => {
			transferProgress = data;
			if (data.done) {
				// Auto-clear after 3 seconds.
				setTimeout(() => { transferProgress = null; }, 3000);
			}
		});

		return () => { unsub(); };
	});

	// Tauri native drag & drop
	$effect(() => {
		if (!browser) return;

		const unlisten = getCurrentWindow().onDragDropEvent(async (event) => {
			if (event.payload.type === 'enter' || event.payload.type === 'over') {
				dragOver = true;
			} else if (event.payload.type === 'leave') {
				dragOver = false;
			} else if (event.payload.type === 'drop') {
				dragOver = false;
				if (!currentPath) return;
				const paths = event.payload.paths;
				if (!paths || paths.length === 0) return;

				loading = true;
				try {
					const count = await FsUploadLocal(paths, currentPath);
					await navigate(currentPath);
					if (count > 0) {
						toast.success('Upload', `${count} file(s) uploaded`);
					} else {
						toast.warning('Upload', 'No files were uploaded');
					}
				} catch (e) {
					toast.error('Upload failed', String(e));
					await navigate(currentPath);
				} finally {
					loading = false;
				}
			}
		});

		return () => { unlisten.then(fn => fn()); };
	});

	// Close context menu on click anywhere
	$effect(() => {
		if (!browser || !contextMenu) return;
		const close = () => { contextMenu = null; };
		window.addEventListener('click', close);
		return () => window.removeEventListener('click', close);
	});

	async function navigate(path: string) {
		loading = true;
		selectedPaths = new Set();
		contextMenu = null;
		try {
			const resp = await FsList(path, showHidden);
			currentPath = resp.path;
			entries = resp.entries;
			truncated = resp.truncated;
		} catch (e) {
			toast.error('File browser', String(e));
		} finally {
			loading = false;
		}
	}

	function goUp() {
		if (!currentPath) return;
		const isWindows = currentPath.includes('\\');
		const sep = isWindows ? '\\' : '/';
		const parts = currentPath.split(sep).filter(Boolean);
		if (parts.length <= 1) return;
		parts.pop();
		navigate(isWindows ? parts.join('\\') : '/' + parts.join('/'));
	}

	function handleEntryDblClick(entry: FsEntry) {
		if (entry.isDir) navigate(entry.path);
	}

	// Selection
	function toggleSelect(entry: FsEntry, event: MouseEvent) {
		const newSet = new Set(selectedPaths);
		if (event.shiftKey && selectedPaths.size > 0) {
			const lastSelected = [...selectedPaths].pop()!;
			const lastIdx = entries.findIndex(e => e.path === lastSelected);
			const currentIdx = entries.findIndex(e => e.path === entry.path);
			const [start, end] = lastIdx < currentIdx ? [lastIdx, currentIdx] : [currentIdx, lastIdx];
			for (let i = start; i <= end; i++) newSet.add(entries[i].path);
		} else if (event.ctrlKey || event.metaKey) {
			if (newSet.has(entry.path)) newSet.delete(entry.path); else newSet.add(entry.path);
		} else {
			if (newSet.has(entry.path) && newSet.size === 1) newSet.clear();
			else { newSet.clear(); newSet.add(entry.path); }
		}
		selectedPaths = newSet;
	}

	function selectAll() {
		selectedPaths = selectedPaths.size === entries.length ? new Set() : new Set(entries.map(e => e.path));
	}

	// Context menu
	function showContextMenu(e: MouseEvent, entry: FsEntry) {
		e.preventDefault();
		// Select entry if not already selected
		if (!selectedPaths.has(entry.path)) {
			selectedPaths = new Set([entry.path]);
		}
		contextMenu = { x: e.clientX, y: e.clientY, entry };
	}

	function contextAction(action: string) {
		if (!contextMenu) return;
		const entry = contextMenu.entry;
		contextMenu = null;

		switch (action) {
			case 'open': handleEntryDblClick(entry); break;
			case 'download': handleDownload(entry); break;
			case 'download-selected': handleDownloadSelected(); break;
			case 'rename': startRename(entry); break;
			case 'delete': handleDelete(entry); break;
			case 'delete-selected': handleDeleteSelected(); break;
		}
	}

	// Actions
	async function handleDownload(entry: FsEntry) {
		loading = true;
		try {
			const count = await FsDownloadPath(entry.path, entry.isDir);
			if (count > 0) {
				toast.success('Download', entry.isDir ? `${count} file(s) downloaded` : `${entry.name} saved`);
			}
		} catch (e) {
			if (String(e) !== '') toast.error('Download failed', String(e));
		} finally {
			loading = false;
		}
	}

	async function handleDownloadSelected() {
		const selected = entries.filter(e => selectedPaths.has(e.path));
		if (selected.length === 0) return;

		if (selected.length === 1) {
			await handleDownload(selected[0]);
			return;
		}

		// Multiple items — single folder picker, batch download.
		loading = true;
		try {
			const paths: [string, boolean][] = selected.map(e => [e.path, e.isDir]);
			const count = await FsDownloadBatch(paths);
			if (count > 0) toast.success('Download', `${count} file(s) downloaded`);
		} catch (e) {
			if (String(e) !== '') toast.error('Download failed', String(e));
		} finally {
			loading = false;
		}
	}

	async function handleUpload() {
		if (!currentPath) return;
		loading = true;
		try {
			await FsUpload(currentPath);
			await navigate(currentPath);
		} catch (e) {
			if (String(e) !== '') toast.error('Upload failed', String(e));
		} finally {
			loading = false;
		}
	}

	async function handleDelete(entry: FsEntry) {
		const what = entry.isDir ? 'directory' : 'file';
		if (!confirm(`Delete ${what} "${entry.name}"?`)) return;
		loading = true;
		try {
			await FsDelete(entry.path);
			await navigate(currentPath);
			toast.success('Deleted', `${entry.name} deleted`);
		} catch (e) {
			toast.error('Delete failed', String(e));
		} finally {
			loading = false;
		}
	}

	async function handleDeleteSelected() {
		const count = selectedPaths.size;
		if (count === 0) return;
		if (!confirm(`Delete ${count} item(s)?`)) return;
		loading = true;
		try {
			for (const path of selectedPaths) await FsDelete(path);
			selectedPaths = new Set();
			await navigate(currentPath);
			toast.success('Deleted', `${count} item(s) deleted`);
		} catch (e) {
			toast.error('Delete failed', String(e));
			await navigate(currentPath);
		} finally {
			loading = false;
		}
	}

	function startRename(entry: FsEntry) {
		renamingEntry = entry;
		renameValue = entry.name;
	}

	async function confirmRename() {
		if (!renamingEntry || !renameValue || renameValue === renamingEntry.name) {
			renamingEntry = null;
			return;
		}
		const isWindows = currentPath.includes('\\');
		const sep = isWindows ? '\\' : '/';
		loading = true;
		try {
			await FsRename(renamingEntry.path, currentPath + sep + renameValue);
			renamingEntry = null;
			await navigate(currentPath);
		} catch (e) {
			toast.error('Rename failed', String(e));
		} finally {
			loading = false;
		}
	}

	function startNewFolder() {
		creatingFolder = true;
		newFolderName = '';
	}

	async function confirmNewFolder() {
		if (!newFolderName) { creatingFolder = false; return; }
		const isWindows = currentPath.includes('\\');
		const sep = isWindows ? '\\' : '/';
		loading = true;
		try {
			await FsMkdir(currentPath + sep + newFolderName);
			creatingFolder = false;
			await navigate(currentPath);
		} catch (e) {
			toast.error('Create folder failed', String(e));
		} finally {
			loading = false;
		}
	}

	function formatSize(bytes: number | undefined): string {
		if (!bytes || bytes === 0) return '-';
		const units = ['B', 'KB', 'MB', 'GB', 'TB'];
		const i = Math.floor(Math.log(bytes) / Math.log(1024));
		return `${(bytes / Math.pow(1024, i)).toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
	}

	function formatDate(epoch: number | undefined): string {
		if (!epoch || epoch === 0) return '';
		return new Date(epoch * 1000).toLocaleString();
	}

	$effect(() => {
		if (!currentPath) navigate('~');
	});
</script>

<div class="space-y-3 h-full">
	<!-- Toolbar -->
	<div class="flex items-center gap-2 flex-wrap">
		<Button size="sm" variant="ghost" onclick={goUp} disabled={loading}>
			<ArrowUp class="w-4 h-4" />
		</Button>
		<Button size="sm" variant="ghost" onclick={() => navigate(currentPath)} disabled={loading}>
			{#if loading}
				<Loader2 class="w-4 h-4 animate-spin" />
			{:else}
				<RefreshCw class="w-4 h-4" />
			{/if}
		</Button>
		<Button size="sm" variant="ghost" onclick={() => { showHidden = !showHidden; navigate(currentPath); }}>
			{#if showHidden}
				<EyeOff class="w-4 h-4" />
			{:else}
				<Eye class="w-4 h-4" />
			{/if}
		</Button>

		<div class="w-px h-5 bg-border"></div>

		<Button size="sm" variant="ghost" onclick={handleUpload} disabled={loading || !currentPath}>
			<Upload class="w-4 h-4 mr-1" />
			Upload
		</Button>
		<Button size="sm" variant="ghost" onclick={startNewFolder} disabled={loading || !currentPath}>
			<FolderPlus class="w-4 h-4 mr-1" />
			New Folder
		</Button>

		{#if selectedCount > 0}
			<div class="w-px h-5 bg-border"></div>
			<span class="text-xs text-muted-foreground">{selectedCount} selected</span>
			<Button size="sm" variant="ghost" onclick={handleDownloadSelected} disabled={loading}>
				<Download class="w-4 h-4 mr-1" />
				Download
			</Button>
			<Button size="sm" variant="ghost" onclick={handleDeleteSelected} disabled={loading}>
				<Trash2 class="w-4 h-4 mr-1 text-red-400" />
				Delete
			</Button>
		{/if}

		<!-- Breadcrumbs -->
		<div class="flex items-center gap-1 text-sm text-muted-foreground overflow-x-auto ml-auto">
			{#each breadcrumbs() as segment, i}
				{#if i > 0}
					<ChevronRight class="w-3 h-3 flex-shrink-0" />
				{/if}
				<button
					type="button"
					class="hover:text-foreground transition-colors whitespace-nowrap"
					onclick={() => navigate(segment.path)}
				>
					{segment.name}
				</button>
			{/each}
		</div>
	</div>

	<!-- New folder input -->
	{#if creatingFolder}
		<div class="flex items-center gap-2 px-3 py-2 rounded-lg border border-primary/50 bg-muted/30">
			<FolderPlus class="w-4 h-4 text-blue-400" />
			<input
				type="text"
				class="flex-1 bg-transparent text-sm outline-none"
				placeholder="Folder name..."
				bind:value={newFolderName}
				onkeydown={(e) => { if (e.key === 'Enter') confirmNewFolder(); if (e.key === 'Escape') creatingFolder = false; }}
				autofocus
			/>
			<Button size="sm" variant="ghost" onclick={confirmNewFolder}>Create</Button>
			<Button size="sm" variant="ghost" onclick={() => creatingFolder = false}>Cancel</Button>
		</div>
	{/if}

	<!-- Drag overlay -->
	{#if dragOver}
		<div class="rounded-lg border-2 border-dashed border-primary/50 bg-primary/5 p-8 text-center">
			<Upload class="w-8 h-8 mx-auto mb-2 text-primary/50" />
			<p class="text-sm text-muted-foreground">Drop files here to upload</p>
		</div>
	{/if}

	<!-- Transfer progress -->
	{#if transferProgress}
		<div class="rounded-lg border border-border bg-muted/30 p-3 space-y-2">
			<div class="flex items-center justify-between text-sm gap-2">
				<span class="text-muted-foreground truncate flex-1">{transferProgress.status}</span>
				<span class="font-mono text-xs whitespace-nowrap">{Math.round(transferProgress.progress * 100)}%</span>
				{#if !transferProgress.done}
					<button
						type="button"
						class="p-1 rounded hover:bg-red-500/20 text-muted-foreground hover:text-red-400 transition-colors"
						onclick={async () => { await FsCancelTransfer(); }}
						title="Cancel transfer"
					>
						<X class="w-4 h-4" />
					</button>
				{/if}
			</div>
			<div class="h-2 w-full rounded-full bg-muted overflow-hidden" style="min-width: 100%;">
				<div
					class="h-full rounded-full {transferProgress.error ? 'bg-red-500' : transferProgress.done ? 'bg-green-500' : 'bg-primary'}"
					style="width: {Math.round(transferProgress.progress * 100)}%; transition: width 0.5s linear;"
				></div>
			</div>
		</div>
	{/if}

	<!-- File list -->
	<div class="rounded-lg border border-border overflow-hidden">
		<table class="w-full text-sm">
			<thead>
				<tr class="border-b border-border bg-muted/50">
					<th class="text-left px-2 py-2 w-8">
						<button type="button" onclick={selectAll} class="p-0.5 hover:text-foreground text-muted-foreground">
							{#if selectedPaths.size === entries.length && entries.length > 0}
								<CheckSquare class="w-4 h-4" />
							{:else}
								<Square class="w-4 h-4" />
							{/if}
						</button>
					</th>
					<th class="text-left px-3 py-2 font-medium">Name</th>
					<th class="text-right px-3 py-2 font-medium w-24">Size</th>
					<th class="text-right px-3 py-2 font-medium w-44 hidden sm:table-cell">Modified</th>
				</tr>
			</thead>
			<tbody>
				{#if entries.length === 0 && !loading}
					<tr>
						<td colspan="4" class="text-center py-8 text-muted-foreground">
							{dragOver ? 'Drop files to upload' : 'Empty directory'}
						</td>
					</tr>
				{/if}
				{#each entries as entry}
					<tr
						class="border-b border-border/50 transition-colors {entry.isDir ? 'cursor-pointer' : ''} {selectedPaths.has(entry.path) ? 'bg-primary/10' : 'hover:bg-muted/30'}"
						ondblclick={() => handleEntryDblClick(entry)}
						onclick={(e) => toggleSelect(entry, e)}
						oncontextmenu={(e) => showContextMenu(e, entry)}
					>
						<td class="px-2 py-1.5 text-center">
							{#if selectedPaths.has(entry.path)}
								<CheckSquare class="w-4 h-4 text-primary" />
							{:else}
								<Square class="w-4 h-4 text-muted-foreground/30" />
							{/if}
						</td>
						<td class="px-3 py-1.5">
							<div class="flex items-center gap-2">
								{#if entry.isDir}
									<Folder class="w-4 h-4 text-blue-400 flex-shrink-0" />
								{:else}
									<File class="w-4 h-4 text-muted-foreground flex-shrink-0" />
								{/if}
								{#if renamingEntry?.path === entry.path}
									<!-- svelte-ignore a11y_autofocus -->
									<input
										type="text"
										class="flex-1 bg-transparent text-sm outline-none border-b border-primary"
										bind:value={renameValue}
										onkeydown={(e) => { if (e.key === 'Enter') confirmRename(); if (e.key === 'Escape') renamingEntry = null; }}
										onclick={(e) => e.stopPropagation()}
										autofocus
									/>
								{:else}
									<span class="truncate" title={entry.name}>{entry.name}</span>
									{#if entry.isSymlink}
										<span class="text-xs text-muted-foreground">&#8594;</span>
									{/if}
								{/if}
							</div>
						</td>
						<td class="text-right px-3 py-1.5 text-muted-foreground whitespace-nowrap">
							{entry.isDir ? '-' : formatSize(entry.size)}
						</td>
						<td class="text-right px-3 py-1.5 text-muted-foreground whitespace-nowrap hidden sm:table-cell">
							{formatDate(entry.modTime)}
						</td>
					</tr>
				{/each}
			</tbody>
		</table>
	</div>

	{#if truncated}
		<p class="text-xs text-yellow-500">Directory listing was truncated (too many entries).</p>
	{/if}
</div>

<!-- Context menu -->
{#if contextMenu}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class="fixed z-50 min-w-[180px] rounded-lg border border-border bg-card shadow-xl py-1 text-sm"
		style="left: {contextMenu.x}px; top: {contextMenu.y}px;"
		onclick={(e) => e.stopPropagation()}
	>
		{#if contextMenu.entry.isDir}
			<button type="button" class="ctx-item" onclick={() => contextAction('open')}>
				<FolderOpen class="w-4 h-4" /> Open
			</button>
		{/if}
		<button type="button" class="ctx-item" onclick={() => contextAction('download')}>
			<Download class="w-4 h-4" /> Download
		</button>
		{#if selectedCount > 1}
			<button type="button" class="ctx-item" onclick={() => contextAction('download-selected')}>
				<Download class="w-4 h-4" /> Download {selectedCount} items
			</button>
		{/if}
		<div class="my-1 border-t border-border"></div>
		<button type="button" class="ctx-item" onclick={() => contextAction('rename')}>
			<Pencil class="w-4 h-4" /> Rename
		</button>
		<button type="button" class="ctx-item text-red-400 hover:text-red-300" onclick={() => contextAction('delete')}>
			<Trash2 class="w-4 h-4" /> Delete
		</button>
		{#if selectedCount > 1}
			<button type="button" class="ctx-item text-red-400 hover:text-red-300" onclick={() => contextAction('delete-selected')}>
				<Trash2 class="w-4 h-4" /> Delete {selectedCount} items
			</button>
		{/if}
	</div>
{/if}

<style>
	.ctx-item {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		width: 100%;
		padding: 0.375rem 0.75rem;
		text-align: left;
		transition: background-color 0.1s;
	}
	.ctx-item:hover {
		background-color: hsl(var(--muted) / 0.5);
	}
</style>
