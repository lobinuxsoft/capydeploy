<script lang="ts">
	import { browser } from '$app/environment';
	import { Card, Badge, Button } from '$lib/components/ui';
	import { GetSteamUsers, GetShortcuts, DeleteShortcut, LaunchGame, EventsOn, EventsOff } from '$lib/wailsjs';
	import { toast } from '$lib/stores/toast';
	import type { SteamUserInfo, ShortcutInfo } from '$lib/types';
	import { Users, Gamepad2, ChevronDown, ChevronRight, Trash2, Loader2, Play } from 'lucide-svelte';

	let users = $state<SteamUserInfo[]>([]);
	let shortcuts = $state<Map<string, ShortcutInfo[]>>(new Map());
	let expandedUsers = $state<Set<string>>(new Set());
	let loading = $state(true);
	let error = $state<string | null>(null);
	let deleting = $state<number | null>(null);
	let expanded = $state(true);

	async function loadUsers() {
		try {
			users = await GetSteamUsers();
			error = null;
		} catch (e) {
			error = e instanceof Error ? e.message : 'Error loading Steam users';
		} finally {
			loading = false;
		}
	}

	async function refreshShortcuts() {
		// Reload shortcuts for all expanded users
		for (const userId of expandedUsers) {
			try {
				const userShortcuts = await GetShortcuts(userId);
				shortcuts.set(userId, userShortcuts);
			} catch (e) {
				console.error('Error refreshing shortcuts:', e);
			}
		}
		shortcuts = new Map(shortcuts);
	}

	async function toggleUser(userId: string) {
		if (expandedUsers.has(userId)) {
			expandedUsers.delete(userId);
			expandedUsers = new Set(expandedUsers);
		} else {
			expandedUsers.add(userId);
			expandedUsers = new Set(expandedUsers);

			// Load shortcuts if not already loaded
			if (!shortcuts.has(userId)) {
				try {
					const userShortcuts = await GetShortcuts(userId);
					shortcuts.set(userId, userShortcuts);
					shortcuts = new Map(shortcuts);
				} catch (e) {
					console.error('Error loading shortcuts:', e);
				}
			}
		}
	}

	async function deleteShortcut(userId: string, shortcut: ShortcutInfo) {
		if (deleting) return;

		deleting = shortcut.appId;
		try {
			await DeleteShortcut(userId, shortcut.appId);
			toast.success('Shortcut deleted', shortcut.name);
		} catch (e) {
			toast.error('Error deleting', e instanceof Error ? e.message : String(e));
		} finally {
			deleting = null;
		}
	}

	$effect(() => {
		if (!browser) return;
		loadUsers();

		// Listen for shortcut changes
		EventsOn('shortcuts:changed', () => {
			refreshShortcuts();
		});

		return () => {
			EventsOff('shortcuts:changed');
			// Cleanup accumulated data
			shortcuts.clear();
			expandedUsers.clear();
		};
	});
</script>

<div class="cd-section p-4">
	<button
		type="button"
		class="w-full flex items-center gap-3 hover:text-primary transition-colors"
		onclick={() => expanded = !expanded}
	>
		{#if expanded}
			<ChevronDown class="w-4 h-4 cd-text-primary" />
		{:else}
			<ChevronRight class="w-4 h-4 cd-text-disabled" />
		{/if}
		<div class="p-2 rounded-lg bg-primary/10">
			<Users class="w-6 h-6 cd-text-primary" />
		</div>
		<div class="text-left">
			<h2 class="cd-section-title text-lg">Steam</h2>
			<p class="text-sm cd-text-disabled">Users and shortcuts</p>
		</div>
	</button>

	{#if expanded}
	{#if loading}
		<div class="flex items-center justify-center py-8 mt-4">
			<div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"></div>
		</div>
	{:else if error}
		<div class="text-center py-8 mt-4 cd-text-destructive">
			<p>{error}</p>
		</div>
	{:else if users.length === 0}
		<div class="text-center py-8 mt-4 cd-text-disabled">
			<p>No Steam users found</p>
		</div>
	{:else}
		<div class="space-y-2 mt-4">
			{#each users as user}
				<div class="rounded-lg border border-border/50 bg-secondary/20 overflow-hidden">
					<button
						type="button"
						class="w-full flex items-center justify-between p-3 hover:bg-secondary/30 transition-colors"
						onclick={() => toggleUser(user.id)}
					>
						<div class="flex items-center gap-2">
							{#if expandedUsers.has(user.id)}
								<ChevronDown class="w-4 h-4 cd-text-primary" />
							{:else}
								<ChevronRight class="w-4 h-4 cd-text-disabled" />
							{/if}
							<span class="cd-value font-medium">{user.name}</span>
						</div>
						<span class="cd-mono text-xs">{user.id}</span>
					</button>

					{#if expandedUsers.has(user.id)}
						<div class="border-t border-border/50 p-3 bg-background/30">
							{#if shortcuts.has(user.id)}
								{@const userShortcuts = shortcuts.get(user.id) || []}
								{#if userShortcuts.length === 0}
									<p class="text-sm cd-text-disabled text-center py-2">
										No shortcuts
									</p>
								{:else}
									<div class="space-y-2">
										{#each userShortcuts as shortcut}
											{@const isDeleting = deleting === shortcut.appId}
											<div class="flex items-center gap-2 p-2 rounded-lg bg-secondary/30 border border-border/30">
												<Gamepad2 class="w-4 h-4 cd-text-capy flex-shrink-0" />
												<div class="flex-1 min-w-0">
													<p class="text-sm cd-value font-medium truncate">{shortcut.name}</p>
													<p class="text-xs cd-text-disabled truncate">{shortcut.exe}</p>
												</div>
												<Badge variant="secondary" class="text-xs flex-shrink-0 cd-mono">
													{shortcut.appId}
												</Badge>
												<Button
													variant="ghost"
													size="icon"
													onclick={() => LaunchGame(shortcut.appId)}
													class="h-7 w-7 flex-shrink-0 text-primary hover:text-primary hover:bg-primary/10"
												>
													<Play class="w-3.5 h-3.5" />
												</Button>
												<Button
													variant="ghost"
													size="icon"
													onclick={() => deleteShortcut(user.id, shortcut)}
													disabled={isDeleting}
													class="h-7 w-7 flex-shrink-0 text-destructive hover:text-destructive hover:bg-destructive/10"
												>
													{#if isDeleting}
														<Loader2 class="w-3.5 h-3.5 animate-spin" />
													{:else}
														<Trash2 class="w-3.5 h-3.5" />
													{/if}
												</Button>
											</div>
										{/each}
									</div>
								{/if}
							{:else}
								<div class="flex items-center justify-center py-2">
									<div class="animate-spin rounded-full h-4 w-4 border-b-2 border-primary"></div>
								</div>
							{/if}
						</div>
					{/if}
				</div>
			{/each}
		</div>
	{/if}
	{/if}
</div>
