<script lang="ts">
	import Icon from '@iconify/svelte';
	import { onMount, onDestroy } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import AppStatusPill from '../../../../components/app-status-pill.svelte';
	import TimeAgo from '../../../../components/time-ago.svelte';
	import LogViewer from '../../../../components/log-viewer.svelte';
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

	// Terminal containers (exited/dead/removing) still expose their retained
	// (historical) logs, but live follow is impossible — they produce no more
	// output. Other states (paused, stopping, ...) can still be followed.
	// NOTE: this list mirrors `ContainerState::is_terminal()` in
	// scotty-core/src/apps/app_data/container.rs — keep the two in sync if the
	// ContainerStatus enum gains a new terminal state.
	$: isTerminal = ['exited', 'dead', 'removing', 'empty'].includes(
		(data.service.status || '').toLowerCase()
	);

	let hasStartedStream = false;
	let range: number | null = 500;

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
			goto(resolve('/dashboard/[slug]', { slug: data.app.name }));
			return;
		}
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
		initializeContainerLogs(data.app.name, data.service.service, range);
		webSocketStore.requestLogStream(data.app.name, data.service.service, true, range, false);
	}

	function changeRange(next: number | null) {
		range = next;
		if (logState.streamId) webSocketStore.stopLogStream(logState.streamId);
		startLogStream();
	}
</script>

<div class="ph">
	<div class="ph-row"><h1>{data.service.service}</h1></div>
	<div class="chips">
		<AppStatusPill status={data.service.status || 'unknown'} />
		{#if data.service.started_at}
			<span class="meta">Started <TimeAgo dateString={data.service.started_at} /></span>
		{/if}
		{#each data.service.domains as domain (domain)}
			<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
			<a
				class="link meta"
				href="{data.service.use_tls ? 'https' : 'http'}://{domain}"
				target="_blank"
				rel="external">{domain}</a
			>
		{/each}
	</div>
</div>

{#if isTerminal}
	<div class="banner">
		<Icon icon="ph:info" class="ic" aria-hidden="true" />
		<div class="txt">
			<b>The container has stopped</b>
			<span
				>Showing the logs it wrote before it exited. Live follow starts again once the app
				runs.</span
			>
		</div>
	</div>
{/if}

{#if logState.error}
	<div class="banner err">
		<Icon icon="ph:warning-circle" class="ic" aria-hidden="true" />
		<div class="txt"><b>Could not load the logs</b><span>{logState.error}</span></div>
	</div>
{/if}

<section class="sec">
	<div class="sec-h">
		<h2>Logs</h2>
		<span class="meta">stdout and stderr, newest at the bottom</span>
	</div>
	<LogViewer
		lines={logState.lines}
		live={logState.streaming && logState.follow}
		stopped={isTerminal}
		{range}
		onRangeChange={changeRange}
	/>
</section>
