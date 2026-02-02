<script lang="ts">
	import { StatusPanel, SteamInfo } from '$lib/components';
</script>

<div class="min-h-screen p-4">
	<header class="mb-6">
		<h1 class="text-2xl font-bold">CapyDeploy Agent</h1>
		<p class="text-sm text-muted-foreground">Recibe juegos desde el Hub</p>
	</header>

	<div class="space-y-4">
		<StatusPanel />
		<SteamInfo />
	</div>
</div>
