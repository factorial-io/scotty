<script>
	import { onMount } from 'svelte';
	import { apps } from '../../stores/appsStore';
	import { loadApps } from '../../stores/appsStore';

	import StartStopAppAction from '../../components/start-stop-app-action.svelte';

	onMount(() => {
		loadApps();
	});
</script>

<h2 class="card-title mb-4 mt-8">List of apps</h2>
<table class="table">
	<!-- head -->
	<thead>
		<tr>
			<th class="w-1"></th>
			<th>Name</th>
			<th>Services</th>
			<th>Status</th>
		</tr>
	</thead>
	<tbody>
		{#each $apps as app}
			<tr>
				<td><StartStopAppAction name={app.name} status={app.status} /></td>
				<td>{app.name}</td>
				<td
					>{#each app.services as service}
						{#if service.domain}
							{#if app.status !== 'Running'}
								<button class="btn btn-xs" disabled="disabled">{service.service}</button>
							{:else}
								<a class="btn btn-xs" href={service.url} target="_">{service.service}</a>
							{/if}
						{/if}
					{/each}</td
				>
				<td>{app.status}</td>
			</tr>
		{/each}
	</tbody>
</table>
