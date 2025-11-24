<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import PageHeader from '../../../../components/page-header.svelte';
	import AppStatusPill from '../../../../components/app-status-pill.svelte';
	import TimeAgo from '../../../../components/time-ago.svelte';
	import UnifiedOutput from '../../../../components/unified-output.svelte';
	import AppServiceButton from '../../../../components/app-service-button.svelte';
	import { setTitle } from '../../../../stores/titleStore';
	import {
		getContainerLogs,
		initializeContainerLogs,
		clearContainerLogs
	} from '../../../../stores/containerLogsStore';
	import { webSocketStore, isConnected } from '../../../../stores/webSocketStore';
	import {
		getAppPermissions,
		permissionsLoaded,
		loadUserPermissions
	} from '../../../../stores/permissionStore';
	import type { App, AppService } from '../../../../types';

	export let data: { app: App; service: AppService };

	// Reactive permissions
	$: permissions = $permissionsLoaded
		? getAppPermissions(data.app.name, ['logs', 'shell'])
		: { logs: false, shell: false };

	// Get the log store for this container
	$: logStore = getContainerLogs(data.app.name, data.service.service);
	$: logState = $logStore;

	let hasStartedStream = false;

	// Start log stream when WebSocket is connected
	$: if ($isConnected && !hasStartedStream && $permissionsLoaded && permissions.logs) {
		startLogStream();
		hasStartedStream = true;
	}

	onMount(async () => {
		setTitle(`Service: ${data.service.service} (${data.app.name})`);

		// Load permissions if needed
		if (!$permissionsLoaded) {
			try {
				await loadUserPermissions();
			} catch (error) {
				console.error('Failed to load permissions:', error);
			}
		}

		// Check permissions and redirect if needed
		if ($permissionsLoaded && !permissions.logs) {
			console.warn('User does not have logs permission, redirecting to app page');
			goto(resolve(`/dashboard/${data.app.name}`));
			return;
		}

		// Initialize log state
		initializeContainerLogs(data.app.name, data.service.service);
	});

	onDestroy(() => {
		// Stop streaming when leaving the page
		if (logState.streamId) {
			webSocketStore.stopLogStream(logState.streamId);
		}
		// Clear logs from store
		clearContainerLogs(data.app.name, data.service.service);
	});

	function startLogStream() {
		console.log('Starting log stream for', data.app.name, data.service.service);
		webSocketStore.requestLogStream(data.app.name, data.service.service, true, null, false);
	}
</script>

<PageHeader>
	<h2 class="card-title" slot="header">
		Service: {data.service.service}
		<span class="text-sm font-normal text-gray-500">({data.app.name})</span>
	</h2>
	<div slot="meta">
		<AppStatusPill status={data.service.status || 'unknown'} />
	</div>
</PageHeader>

<!-- Breadcrumb Navigation -->
<div class="text-sm breadcrumbs mb-4">
	<ul>
		<li><a href={resolve('/dashboard')}>Dashboard</a></li>
		<li><a href={resolve(`/dashboard/${data.app.name}`)}>{data.app.name}</a></li>
		<li>{data.service.service}</li>
	</ul>
</div>

<!-- Service Information Section -->
<h3 class="text-xl mt-8 mb-4">Service Information</h3>
<table class="table table-zebra">
	<tbody>
		<tr>
			<td class="text-gray-500 w-48"><strong>Service Name</strong></td>
			<td>{data.service.service}</td>
		</tr>
		<tr>
			<td class="text-gray-500"><strong>Status</strong></td>
			<td><AppStatusPill status={data.service.status || 'unknown'} /></td>
		</tr>
		<tr>
			<td class="text-gray-500"><strong>Started</strong></td>
			<td><TimeAgo dateString={data.service.started_at} /></td>
		</tr>
		<tr>
			<td class="text-gray-500"><strong>Domain(s)</strong></td>
			<td>
				{#if data.service.domains && data.service.domains.length > 0}
					<AppServiceButton
						property="domain"
						service={data.service}
						status={data.app.status}
					/>
				{:else}
					--
				{/if}
			</td>
		</tr>
	</tbody>
</table>

<!-- Log Viewer Section -->
<h3 class="text-xl mt-8 mb-4">Container Logs</h3>

{#if logState.error}
	<div class="alert alert-error text-sm mb-4">
		<span>⚠️ Error: {logState.error}</span>
	</div>
{/if}

<!-- Unified Output Component -->
<UnifiedOutput lines={logState.lines} heading="" loading={logState.loading} maxHeight="600px" />
