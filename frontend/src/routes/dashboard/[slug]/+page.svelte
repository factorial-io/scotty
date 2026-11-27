<script lang="ts">
	import Icon from '@iconify/svelte';
	import { resolve } from '$app/paths';
	import AppStatusPill from '../../../components/app-status-pill.svelte';
	import AppConnectivityPill from '../../../components/app-connectivity-pill.svelte';
	import TimeAgo from '../../../components/time-ago.svelte';
	import { dispatchAppCommand } from '../../../stores/appsStore';
	import { goto } from '$app/navigation';
	import {
		getAppPermissions,
		permissionsLoaded,
		permissionsLoading,
		loadUserPermissions
	} from '../../../stores/permissionStore';
	import type { App, TaskDetail } from '../../../types';
	import { time } from '../../../stores/time';
	import { endOfLife, formatLifetime, formatTtl } from '../../../lib/lifetime';
	import TasksTable from '../../../components/tasks-table.svelte';
	import { tasks } from '../../../stores/tasksStore';
	import { getApp, apps } from '../../../stores/appsStore';
	import FormatBasicAuth from '../../../components/format-basic-auth.svelte';
	import FormatEnvironmentVariables from '../../../components/format-environment-variables.svelte';
	import { onDestroy, onMount } from 'svelte';
	import { setTitle } from '../../../stores/titleStore';
	import { showError } from '../../../stores/errorStore';
	import { DropdownMenu } from 'bits-ui';
	import { askConfirm } from '../../../stores/confirmStore';
	import CustomActionsDropdown from '../../../components/custom-actions-dropdown.svelte';
	import TaskProgress from '../../../components/task-progress.svelte';
	import { operationTitle } from '../../../lib/operationTitle';

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
		? getAppPermissions(data, ['view', 'manage', 'destroy', 'shell', 'logs'])
		: { view: false, manage: false, destroy: false, shell: false, logs: false };

	$: isLoadingPermissions = $permissionsLoading || !$permissionsLoaded;

	// Set between the click and the server's response; afterwards
	// `data.operation` says whether the app is busy.
	let current_action: string | null = null;

	$: busy = current_action !== null || !!data.operation;
	$: running = data.status === 'Running';
	$: canDestroy = permissions.destroy && !!data.settings;

	async function handleClick(action: string) {
		if (current_action) return;
		current_action = action;
		try {
			await dispatchAppCommand(action.toLowerCase(), data.name);
		} catch (err) {
			showError(`${action} failed`, err);
		} finally {
			current_action = null;
		}
	}

	async function destroy() {
		const n = data.services.length;
		const ok = await askConfirm({
			title: `Destroy ${data.name}?`,
			body: 'Scotty stops the app and deletes it from this server.',
			consequence: `This removes ${n === 1 ? '1 service and its' : `${n} services and all their`} data, including volumes. It cannot be undone.`,
			confirmLabel: 'Destroy app'
		});
		if (ok) handleClick('Destroy');
	}

	function isSupported() {
		return data.status !== 'Unsupported';
	}

	let app_tasks: TaskDetail[] = [];
	onDestroy(
		tasks.subscribe((new_tasks) => {
			app_tasks = Object.values(new_tasks)
				.filter((t) => t.app_name === data.name)
				.sort((a, b) => b.start_time.localeCompare(a.start_time));
		})
	);

	let seenInStore = false;
	onDestroy(
		apps.subscribe(() => {
			let result = getApp(data.name);
			if (result) {
				seenInStore = true;
				data = result;
			} else if (seenInStore) {
				// The app was destroyed.
				goto(resolve('/dashboard'));
			}
		})
	);

	function format_value(value: unknown) {
		return value === null || value === undefined || value === '' ? 'None' : value;
	}

	const cap = (s: string) => s[0].toUpperCase() + s.slice(1);
</script>

<div class="ph">
	<div class="ph-row">
		<h1>{data.name}</h1>
		{#if isSupported()}
			<div class="acts">
				{#if isLoadingPermissions}
					<span class="meta">Loading permissions…</span>
				{:else}
					{#if permissions.manage}
						<button
							class="btn filled primary"
							disabled={busy}
							on:click={() => handleClick(running ? 'Stop' : 'Run')}
						>
							<Icon
								icon={busy && current_action !== null
									? 'ph:spinner'
									: running
										? 'ph:stop'
										: 'ph:play'}
								class="ic sm {busy && current_action !== null ? 'spin' : ''}"
								aria-hidden="true"
							/>{running ? 'Stop' : 'Run'}</button
						>
						<button
							class="btn outline"
							disabled={busy}
							on:click={() => handleClick('Rebuild')}
						>
							<Icon
								icon="ph:hammer"
								class="ic sm"
								aria-hidden="true"
							/>Rebuild</button
						>
						<button
							class="btn outline"
							disabled={busy}
							on:click={() => handleClick('Purge')}
						>
							<Icon icon="ph:broom" class="ic sm" aria-hidden="true" />Purge</button
						>
					{/if}
					<CustomActionsDropdown app={data} canManage={permissions.manage} />
					{#if canDestroy}
						<DropdownMenu.Root>
							<DropdownMenu.Trigger class="ibtn" aria-label="More actions">
								<Icon icon="ph:dots-three" class="ic" aria-hidden="true" />
							</DropdownMenu.Trigger>
							<DropdownMenu.Portal>
								<DropdownMenu.Content class="menu" align="end" sideOffset={6}>
									<DropdownMenu.Item
										disabled={busy}
										onSelect={destroy}
										class="danger"
									>
										<Icon
											icon="ph:trash"
											class="ic sm"
											aria-hidden="true"
										/>Destroy app…
									</DropdownMenu.Item>
								</DropdownMenu.Content>
							</DropdownMenu.Portal>
						</DropdownMenu.Root>
					{/if}
				{/if}
			</div>
		{/if}
	</div>
	<div class="chips">
		<AppStatusPill status={data.status} />
		<AppConnectivityPill connectivity={data.load_balancer_connectivity} />
		{#if data.settings?.time_to_live}
			<span class="tag"
				><Icon icon="ph:clock" class="ic sm" aria-hidden="true" />{formatLifetime(
					data,
					$time
				)}</span
			>
		{/if}
		{#if data.settings?.app_blueprint}
			<span class="tag">Blueprint: {data.settings.app_blueprint}</span>
		{/if}
		{#if data.next_check}
			<span class="meta">Next update <TimeAgo dateString={data.next_check} /></span>
		{/if}
	</div>
</div>

{#if !isSupported()}
	<div class="banner warn">
		<Icon icon="ph:warning" class="ic" aria-hidden="true" />
		<div class="txt">
			<b>Scotty does not manage this app</b>
			<span
				>It was started outside of Scotty, so it can only be listed. Recreate it with <span
					class="mono">scottyctl app:create</span
				> to manage it here.</span
			>
		</div>
	</div>
{/if}

{#if data.operation}
	<div class="banner">
		<Icon icon="ph:spinner" class="ic spin" aria-hidden="true" />
		<div class="txt">
			<b
				>{operationTitle(data.operation.operation, data.name) ??
					`${cap(data.operation.operation)} running`}, started <TimeAgo
					dateString={data.operation.started_at}
				/></b
			>
			<span
				>Actions are paused until it finishes.
				{#if data.operation.task_id}
					<a
						class="link"
						href={resolve('/tasks/[slug]', { slug: data.operation.task_id })}
						>Follow the task output</a
					>
				{/if}</span
			>
			{#if data.operation.task_id}
				<TaskProgress taskId={data.operation.task_id} details />
			{/if}
		</div>
	</div>
{/if}

<section class="sec">
	<div class="sec-h">
		<h2>Services</h2>
		<span class="meta"
			>{data.services.length} {data.services.length === 1 ? 'container' : 'containers'}</span
		>
	</div>
	<div class="tw">
		<table>
			<thead>
				<tr><th>Name</th><th>Status</th><th>URL</th><th class="num">Started</th></tr>
			</thead>
			<tbody>
				{#each data.services as service (service.service)}
					<tr>
						<td>
							{#if permissions.logs}
								<a
									class="link name"
									href={resolve('/dashboard/[slug]/[service]', {
										slug: data.name,
										service: service.service
									})}>{service.service}</a
								>
							{:else}
								{service.service}
							{/if}
						</td>
						<td><AppStatusPill status={service.status || 'unknown'} /></td>
						<td>
							{#each service.domains as domain (domain)}
								<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
								<a
									class="link"
									class:muted={service.status !== 'Running'}
									href="{service.use_tls ? 'https' : 'http'}://{domain}"
									target="_blank"
									rel="external">{domain}</a
								><br />
							{:else}
								<span class="meta">—</span>
							{/each}
						</td>
						<td class="num mono"><TimeAgo dateString={service.started_at} /></td>
					</tr>
				{/each}
			</tbody>
		</table>
	</div>
</section>

{#if data.settings}
	<section class="sec">
		<div class="sec-h"><h2>Settings</h2></div>
		<dl class="dl">
			<div>
				<dt>Blueprint</dt>
				<dd>{format_value(data.settings.app_blueprint)}</dd>
			</div>
			<div>
				<dt>Basic auth</dt>
				<dd><FormatBasicAuth auth={data.settings.basic_auth} /></dd>
			</div>
			<div>
				<dt>Disallow robots</dt>
				<dd>{data.settings.disallow_robots ? 'Yes' : 'No'}</dd>
			</div>
			<div>
				<dt>Private registry</dt>
				<dd>{format_value(data.settings.registry)}</dd>
			</div>
			<div>
				<dt>Time to live</dt>
				<dd>{formatTtl(data.settings.time_to_live)}, then {endOfLife(data)}</dd>
			</div>
			<div>
				<dt>Middlewares</dt>
				<dd>{format_value(data.settings.middlewares?.join(', '))}</dd>
			</div>
			<div>
				<dt>Blocked paths</dt>
				<dd>
					{format_value(data.blocked_paths?.join(', '))}
					{#if data.blocked_paths?.length}
						{data.blocked_paths_enforced
							? '(current config, applies after the next rebuild)'
							: '(not enforced: the load balancer is not Traefik)'}
					{/if}
				</dd>
			</div>
			<div>
				<dt>Environment</dt>
				<dd><FormatEnvironmentVariables environment={data.settings.environment} /></dd>
			</div>
		</dl>
	</section>
{/if}

{#if app_tasks.length > 0}
	<section class="sec">
		<div class="sec-h">
			<h2>Latest tasks</h2>
			<a class="link meta" href={resolve('/tasks')}>All tasks</a>
		</div>
		<TasksTable taskList={app_tasks} withApp={false} />
	</section>
{/if}
