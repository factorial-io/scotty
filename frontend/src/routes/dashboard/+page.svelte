<script lang="ts">
	import { onMount } from 'svelte';
	import { apps } from '../../stores/appsStore';
	import { loadApps } from '../../stores/appsStore';
	import { writable } from 'svelte/store';
	import MagnifyingGlass from '@iconify-icons/ph/magnifying-glass';
	import { setTitle } from '../../stores/titleStore';

	import StartStopAppAction from '../../components/start-stop-app-action.svelte';
	import AppServiceButton from '../../components/app-service-button.svelte';
	import AppStatusPill from '../../components/app-status-pill.svelte';
	import Icon from '@iconify/svelte';
	import PageHeader from '../../components/page-header.svelte';
	import type { App } from '../../types';
	import TableHeaderSort from '../../components/table-header-sort.svelte';
	import LastStarted from '../../components/last-started.svelte';

	const filter = writable('');
	const sortBy = writable('status');

	function setSort(event: CustomEvent<{ sortBy: string }>) {
		sortBy.set(event.detail.sortBy);
	}

	function doSort(a: App, b: App): number {
		if ($sortBy === 'name') {
			return a.name.localeCompare(b.name);
		}
		if ($sortBy === 'status') {
			const result = a.status.localeCompare(b.status);
			if (result === 0) {
				return a.name.localeCompare(b.name);
			} else {
				return result;
			}
		}
		return 0;
	}

	const filteredAndSortedApps = writable([] as App[]);
	function updateAppList() {
		$filteredAndSortedApps = $apps
			.filter((app) => app.name.toLowerCase().includes($filter.toLowerCase()))
			.sort((a, b) => doSort(a, b));
	}

	apps.subscribe(() => {
		updateAppList();
	});

	sortBy.subscribe(() => {
		updateAppList();
	});
	filter.subscribe(() => {
		updateAppList();
	});

	onMount(async () => {
		setTitle('Apps Dashboard');
		await loadApps();
		updateAppList();
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
			<th
				><TableHeaderSort on:setSort={setSort} sortBy={$sortBy} key="name" name="Name"
				></TableHeaderSort></th
			>
			<th>Services</th>
			<th
				><TableHeaderSort on:setSort={setSort} sortBy={$sortBy} key="status" name="Status"
				></TableHeaderSort></th
			>
			<th>Last time started</th>
		</tr>
	</thead>
	<tbody>
		{#each $filteredAndSortedApps as app (app.name)}
			<tr>
				<td><StartStopAppAction name={app.name} status={app.status} /></td>
				<td><a class="link-primary" href="/dashboard/{app.name}">{app.name}</a></td>
				<td
					>{#each app.services as service (service.service)}
						{#if service.domains && service.domains.length > 0}
							<AppServiceButton status={app.status} {service} />
						{/if}
					{/each}</td
				>
				<td><AppStatusPill status={app.status} /></td>
				<td><LastStarted services={app.services}></LastStarted> </td></tr
			>
		{/each}
	</tbody>
</table>
