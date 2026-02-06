<script lang="ts">
	import { StatusPanel, SteamInfo, ProgressOverlay, AuthorizedHubs } from '$lib/components';
</script>

<ProgressOverlay />

<div class="min-h-screen">
	<!-- Header with mascot (Decky style) -->
	<div class="m-4 mb-4">
		<div class="cd-header">
			<div class="cd-mascot-wrap">
				<img src="/mascot.gif" alt="CapyDeploy" />
			</div>
			<div class="flex-1">
				<h1 class="cd-title">CapyDeploy Agent</h1>
				<p class="cd-subtitle">Recibe juegos desde el Hub</p>
			</div>
		</div>
	</div>

	<div class="px-4 space-y-4">
		<StatusPanel />
		<AuthorizedHubs />
		<SteamInfo />
	</div>
</div>
