<script>
	import { apiCall } from '$lib';
	import { onMount } from 'svelte';

	let apps = [];

	$: {
		onMount(async () => {
			async function getApps() {
				const response = await apiCall('apps/list');
				let result = await response.json();
				return result.apps || [];
			}

			apps = await getApps();
			console.log(apps);
		});
	}
</script>

<h2 class="card-title">List of apps</h2>
<table class="table">
	<!-- head -->
	<thead>
		<tr>
			<th>Name</th>
			<th>Services</th>
			<th>Status</th>
		</tr>
	</thead>
	<tbody>
		{#each apps as app}
			<tr>
				<td>{app.name}</td>
				<td
					>{#each app.services as service}
						{#if service.domain}
							{#if app.status !== 'Running'}
								<button class="btn btn-sm" disabled="disabled">{service.service}</button>
							{:else}
								<a class="btn btn-sm" href={service.url} target="_">{service.service}</a>
							{/if}
						{/if}
					{/each}</td
				>
				<td>{app.status}</td>
			</tr>
		{/each}
	</tbody>
</table>
