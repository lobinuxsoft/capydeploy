<script lang="ts">
	import { StatusPanel, SteamInfo, ProgressOverlay, AuthorizedHubs } from '$lib/components';
</script>

<ProgressOverlay />

<div class="min-h-screen">
	<!-- Header with mascot -->
	<header class="flex items-center gap-3 px-4 py-3 glass-card border-b border-border mb-4">
		<div class="mini-mascot">
			<img src="/mascot.gif" alt="CapyDeploy" />
		</div>
		<div>
			<h1 class="text-lg font-bold gradient-text">CapyDeploy Agent</h1>
			<p class="text-xs text-muted-foreground">Recibe juegos desde el Hub</p>
		</div>
	</header>

	<div class="px-4 space-y-4">
		<StatusPanel />
		<AuthorizedHubs />
		<SteamInfo />
	</div>
</div>
