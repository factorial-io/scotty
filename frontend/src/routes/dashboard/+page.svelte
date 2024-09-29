<script>
	import { onMount } from 'svelte';
	import { apps } from '../../stores/appsStore';
	import { loadApps } from '../../stores/appsStore';
	import { writable } from 'svelte/store';
	import MagnifyingGlass from '@iconify-icons/heroicons/magnifying-glass';

	import StartStopAppAction from '../../components/start-stop-app-action.svelte';
	import AppServiceButton from '../../components/app-service-button.svelte';
	import AppStatusPill from '../../components/app-status-pill.svelte';
	import Icon from '@iconify/svelte';
	import PageHeader from '../../components/page-header.svelte';

	const filter = writable('');

	onMount(() => {
		loadApps();
	});
</script>

<PageHeader>
	<h2 slot="header" class="card-title">List of apps</h2>
	<label slot="meta" class="input input-bordered input-sm flex items-center gap-2">
		<input type="text" bind:value={$filter} placeholder="Filter by name" class="grow" />
		<Icon icon={MagnifyingGlass} />
	</label>
</PageHeader>
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
		{#each $apps
			.filter((app) => app.name.toLowerCase().includes($filter.toLowerCase()))
			.sort((a, b) => a.name.localeCompare(b.name)) as app}
			<tr>
				<td><StartStopAppAction name={app.name} status={app.status} /></td>
				<td><a class="link-primary" href="/dashboard/{app.name}">{app.name}</a></td>
				<td
					>{#each app.services as service}
						{#if service.domain}
							<AppServiceButton status={app.status} {service} />
						{/if}
					{/each}</td
				>
				<td><AppStatusPill status={app.status} /></td>
			</tr>
		{/each}
	</tbody>
</table>
