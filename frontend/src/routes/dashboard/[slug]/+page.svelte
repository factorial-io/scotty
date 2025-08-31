<script lang="ts">
	import PageHeader from '../../../components/page-header.svelte';
	import AppStatusPill from '../../../components/app-status-pill.svelte';
	import TimeAgo from '../../../components/time-ago.svelte';
	import { dispatchAppCommand, updateAppInfo } from '../../../stores/appsStore';
	import { monitorTask } from '../../../stores/tasksStore';
	import {
		getAppPermissions,
		permissionsLoaded,
		permissionsLoading,
		loadUserPermissions
	} from '../../../stores/permissionStore';
	import type { App, AppTtl, TaskDetail } from '../../../types';
	import TasksTable from '../../../components/tasks-table.svelte';
	import { tasks } from '../../../stores/tasksStore';
	import { getApp, apps } from '../../../stores/appsStore';
	import AppServiceButton from '../../../components/app-service-button.svelte';
	import FormatBasicAuth from '../../../components/format-basic-auth.svelte';
	import FormatEnvironmentVariables from '../../../components/format-environment-variables.svelte';
	import { onMount } from 'svelte';
	import { setTitle } from '../../../stores/titleStore';
	import Pill from '../../../components/pill.svelte';
	import CustomActionsDropdown from '../../../components/custom-actions-dropdown.svelte';

	/** @type {import('./$types').PageData} */
	export let data: App;

	onMount(async () => {
		setTitle(`App: ${data.name}`);

		// Ensure permissions are loaded when page is accessed directly
		if (!$permissionsLoaded) {
			try {
				await loadUserPermissions();
			} catch (error) {
				console.error('Failed to load permissions:', error);
			}
		}
	});

	$: permissions = $permissionsLoaded
		? getAppPermissions(data.name, ['view', 'manage', 'destroy', 'shell', 'logs'])
		: { view: false, manage: false, destroy: false, shell: false, logs: false };

	$: isLoadingPermissions = $permissionsLoading || !$permissionsLoaded;

	// Calculate available actions after permissions are loaded
	$: availableActions = (() => {
		let actions: string[] = [];

		if (permissions.manage) {
			actions.push('Run', 'Stop', 'Purge', 'Rebuild');
		}

		if (permissions.destroy && data.settings) {
			actions.push('Destroy');
		}

		return actions;
	})();

	// Debug logging
	$: {
		console.log('App detail page - Permission state:', {
			permissionsLoaded: $permissionsLoaded,
			permissionsLoading: $permissionsLoading,
			isLoadingPermissions,
			permissions,
			'permissions.manage': permissions.manage,
			'permissions.destroy': permissions.destroy,
			'data.settings': !!data.settings,
			availableActions,
			'availableActions.length': availableActions.length
		});
	}

	function getAvailableActions(): string[] {
		let actions: string[] = [];

		if (permissions.manage) {
			actions.push('Run', 'Stop', 'Purge', 'Rebuild');
		}

		if (permissions.destroy && data.settings) {
			actions.push('Destroy');
		}

		return actions;
	}

	let current_task: string | null = null;
	let current_action: string | null = null;
	let customActionsAvailable: boolean = false;

	async function handleClick(action: string) {
		if (action === 'Destroy') {
			if (
				!confirm(`Are you sure you want to destroy the app ${data.name} and all its data?`)
			) {
				return;
			}
		}
		if (current_action) {
			return;
		}
		current_action = action;
		current_task = await dispatchAppCommand(action.toLowerCase(), data.name);
		monitorTask(current_task, async () => {
			current_task = null;
			current_action = null;
			data = (await updateAppInfo(data.name)) as App;
		});
	}

	function isSupported() {
		return data.status !== 'Unsupported';
	}

	let app_tasks: TaskDetail[] = [];
	tasks.subscribe((new_tasks) => {
		app_tasks = Object.values(new_tasks).filter((t) => t.app_name === data.name);
	});

	apps.subscribe(() => {
		let result = getApp(data.name);
		if (result) {
			data = result;
		}
	});

	function format_ttl(ttlData: AppTtl) {
		if (typeof ttlData === 'string') {
			return ttlData;
		} else if (ttlData.Days) {
			return `${ttlData.Days} days`;
		} else {
			return `${ttlData.Hours} hours`;
		}
	}

	function format_value(value: unknown) {
		if (value === null) {
			return 'None';
		}
		return value;
	}
</script>

<PageHeader>
	<h2 class="card-title" slot="header">App-Details for {data.name}</h2>
	<div slot="meta">
		<AppStatusPill status={data.status} />
	</div>
</PageHeader>

<h3 class="text-xl mt-16 mb-4">Available Actions</h3>
<div class="flex flex-wrap items-center gap-2">
	{#if isLoadingPermissions}
		<div class="btn btn-sm join-item loading">Loading permissions...</div>
	{:else}
		<div class="join">
			{#each availableActions as action (action)}
				<button
					disabled={current_task !== null || !isSupported()}
					class="btn btn-sm join-item"
					on:click={() => handleClick(action)}
					>{#if action === current_action}
						<span class="loading loading-spinner"></span>
					{/if}{action}</button
				>
			{/each}
		</div>
		{#if customActionsAvailable && availableActions.length > 0}
			<div class="divider divider-horizontal mx-0"></div>
		{/if}
		<CustomActionsDropdown
			app={data}
			canManage={permissions.manage}
			bind:hasActions={customActionsAvailable}
		/>
	{/if}
</div>
<h3 class="text-xl mt-16 mb-4">Available Services</h3>
<table class="table">
	<thead>
		<tr>
			<th>Name</th>
			<th>Status</th>
			<th>Url(s)</th>
			<th>Started</th>
		</tr>
	</thead>
	<tbody>
		{#each data.services as service (service.service)}
			<tr>
				<td>{service.service}</td>
				<td><AppStatusPill status={service.status || 'unknown'} /></td>
				<td
					>{#if service.domains && service.domains.length > 0}
						<AppServiceButton property="domain" {service} status={data.status} />
					{:else}
						--
					{/if}
				</td>
				<td>
					<TimeAgo dateString={service.started_at} />
				</td>
			</tr>{/each}
	</tbody>
</table>
{#if data.settings}
	<h3 class="text-xl mt-16 mb-4">Settings</h3>
	<table class="table">
		<tbody>
			<tr>
				<td class="text-gray-500 align-top"><strong>App Blueprint</strong></td>
				<td class="align-top">{format_value(data.settings.app_blueprint)}</td>
			</tr>
			<tr>
				<td class="text-gray-500 align-top"><strong>Basic Auth</strong></td>
				<td class="align-top"><FormatBasicAuth auth={data.settings.basic_auth} /></td>
			</tr>
			<tr>
				<td class="text-gray-500 align-top"><strong>Disallow Robots</strong></td>
				<td class="align-top">{data.settings.disallow_robots}</td>
			</tr>
			<tr>
				<td class="text-gray-500 align-top"><strong>Private Docker Registry</strong></td>
				<td class="align-top">{format_value(data.settings.registry)}</td>
			</tr>
			<tr>
				<td class="text-gray-500 align-top"><strong>Time to live</strong></td>
				<td class="align-top">{format_ttl(data.settings.time_to_live)}</td>
			</tr>
			<tr>
				<td class="text-gray-500 align-top"><strong>Middlewares</strong></td>
				<td class="align-top">{format_value(data.settings.middlewares?.join(', '))}</td>
			</tr>
			<tr>
				<td class="text-gray-500 align-top"><strong>Environment</strong></td>
				<td class="align-top"
					><FormatEnvironmentVariables environment={data.settings.environment} /></td
				>
			</tr>
		</tbody>
	</table>
{/if}
{#if app_tasks.length > 0}
	<h3 class="text-xl mt-16 mb-4">Latest task-invocations</h3>
	<TasksTable taskList={app_tasks} />
{/if}
<div class="text-right mt-8">
	<Pill>
		Updated <TimeAgo dateString={data.last_checked} />
	</Pill>
</div>
