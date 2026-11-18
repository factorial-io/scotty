<script lang="ts">
	import { onMount } from 'svelte';
	import { resolve } from '$app/paths';
	import Icon from '@iconify/svelte';
	import { apps, loadApps } from '../../stores/appsStore';
	import { setTitle } from '../../stores/titleStore';
	import StartStopAppAction from '../../components/start-stop-app-action.svelte';
	import AppServiceButton from '../../components/app-service-button.svelte';
	import AppStatusPill from '../../components/app-status-pill.svelte';
	import TableHeaderSort from '../../components/table-header-sort.svelte';
	import LastStarted from '../../components/last-started.svelte';
	import TaskProgress from '../../components/task-progress.svelte';
	import {
		STATUS_FILTERS,
		countByFilter,
		listApps,
		type SortKey,
		type StatusFilter
	} from '../../lib/appList';

	let search = $state('');
	let statusFilter = $state<StatusFilter>('All');
	let sortBy = $state<SortKey>('status');

	let counts = $derived(countByFilter($apps));
	let list = $derived(listApps($apps, search, statusFilter, sortBy));

	function clearFilter() {
		search = '';
		statusFilter = 'All';
	}

	onMount(() => {
		setTitle('Apps');
		loadApps();
	});
</script>

<div class="ph">
	<div class="ph-row"><h1>Apps</h1></div>
	<p class="lead">
		Every app this Scotty server knows about. Start, stop and open them from here; create new
		ones with <span class="mono">scottyctl app:create</span>.
	</p>
</div>

<div class="sec">
	<div class="toolbar">
		<div class="pills" role="group" aria-label="Filter by status">
			{#each STATUS_FILTERS as f (f)}
				<button
					class="pill"
					aria-pressed={statusFilter === f}
					onclick={() => (statusFilter = f)}
					>{f}<span class="mono">{counts[f]}</span></button
				>
			{/each}
		</div>
		<label class="field">
			<Icon icon="ph:magnifying-glass" class="ic" aria-hidden="true" />
			<input
				id="app-search"
				type="search"
				placeholder="Filter by name"
				aria-label="Filter by name"
				bind:value={search}
			/>
		</label>
	</div>

	{#if list.length}
		<div class="tw">
			<table>
				<thead>
					<tr>
						<th style="width:1px"><span class="sr-only">Action</span></th>
						<th
							><TableHeaderSort
								onsetSort={(e) => (sortBy = e.sortBy as SortKey)}
								{sortBy}
								key="name"
								name="Name"
							/></th
						>
						<th>Services</th>
						<th
							><TableHeaderSort
								onsetSort={(e) => (sortBy = e.sortBy as SortKey)}
								{sortBy}
								key="status"
								name="Status"
							/></th
						>
						<th class="num">Last started</th>
					</tr>
				</thead>
				<tbody>
					{#each list as app (app.name)}
						{@const linked = app.services.filter((s) => s.domains.length > 0)}
						<tr>
							<td><StartStopAppAction {app} /></td>
							<td
								><a
									class="name"
									href={resolve('/dashboard/[slug]', { slug: app.name })}
									>{app.name}</a
								></td
							>
							<td>
								<div class="svcs">
									{#each linked as service (service.service)}
										<AppServiceButton {service} />
									{:else}
										<span class="meta">—</span>
									{/each}
								</div>
							</td>
							<td>
								<AppStatusPill status={app.status} />
								{#if app.operation?.task_id}
									<TaskProgress
										taskId={app.operation.task_id}
										kind={app.operation.operation}
									/>
								{/if}
							</td>
							<td class="num mono"><LastStarted services={app.services} /></td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{:else if $apps.length}
		<div class="empty">
			<h2>No app matches “{search || statusFilter}”</h2>
			<p>Check the spelling or show all apps.</p>
			<button class="link" onclick={clearFilter}>Clear filter</button>
		</div>
	{/if}
</div>
