<script lang="ts">
	import { browser } from '$app/environment';
	import { Card, Badge } from '$lib/components/ui';
	import { GetSteamUsers, GetShortcuts, EventsOn, EventsOff } from '$lib/wailsjs';
	import type { SteamUserInfo, ShortcutInfo } from '$lib/types';
	import { Users, Gamepad2, ChevronDown, ChevronRight } from 'lucide-svelte';

	let users = $state<SteamUserInfo[]>([]);
	let shortcuts = $state<Map<string, ShortcutInfo[]>>(new Map());
	let expandedUsers = $state<Set<string>>(new Set());
	let loading = $state(true);
	let error = $state<string | null>(null);

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

	$effect(() => {
		if (!browser) return;
		loadUsers();

		// Listen for shortcut changes
		EventsOn('shortcuts:changed', () => {
			refreshShortcuts();
		});

		return () => {
			EventsOff('shortcuts:changed');
		};
	});
</script>

<Card class="p-6">
	<div class="flex items-center gap-3 mb-6">
		<div class="p-2 rounded-lg bg-primary/10">
			<Users class="w-6 h-6 text-primary" />
		</div>
		<div>
			<h2 class="text-xl font-semibold gradient-text">Steam</h2>
			<p class="text-sm text-muted-foreground">Usuarios y shortcuts</p>
		</div>
	</div>

	{#if loading}
		<div class="flex items-center justify-center py-8">
			<div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"></div>
		</div>
	{:else if error}
		<div class="text-center py-8 text-destructive">
			<p>{error}</p>
		</div>
	{:else if users.length === 0}
		<div class="text-center py-8 text-muted-foreground">
			<p>No se encontraron usuarios de Steam</p>
		</div>
	{:else}
		<div class="space-y-2">
			{#each users as user}
				<div class="rounded-lg border bg-secondary/30 overflow-hidden">
					<button
						type="button"
						class="w-full flex items-center justify-between p-3 hover:bg-secondary/50 transition-colors"
						onclick={() => toggleUser(user.id)}
					>
						<div class="flex items-center gap-2">
							{#if expandedUsers.has(user.id)}
								<ChevronDown class="w-4 h-4" />
							{:else}
								<ChevronRight class="w-4 h-4" />
							{/if}
							<span class="font-medium">{user.name}</span>
						</div>
						<span class="text-xs text-muted-foreground font-mono">{user.id}</span>
					</button>

					{#if expandedUsers.has(user.id)}
						<div class="border-t p-3 bg-background/50">
							{#if shortcuts.has(user.id)}
								{@const userShortcuts = shortcuts.get(user.id) || []}
								{#if userShortcuts.length === 0}
									<p class="text-sm text-muted-foreground text-center py-2">
										Sin shortcuts
									</p>
								{:else}
									<div class="space-y-2">
										{#each userShortcuts as shortcut}
											<div class="flex items-center gap-2 p-2 rounded bg-secondary/50">
												<Gamepad2 class="w-4 h-4 text-muted-foreground" />
												<div class="flex-1 min-w-0">
													<p class="text-sm font-medium truncate">{shortcut.name}</p>
													<p class="text-xs text-muted-foreground truncate">{shortcut.exe}</p>
												</div>
												<Badge variant="secondary" class="text-xs">
													{shortcut.appId}
												</Badge>
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
</Card>
