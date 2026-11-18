<script lang="ts">
	import { onMount } from 'svelte';
	import Icon from '@iconify/svelte';
	import { setTitle } from '../../stores/titleStore';
	import { isInitialized, isLoggedIn } from '../../stores/userStore';
	import { askConfirm } from '../../stores/confirmStore';
	import { time } from '../../stores/time';
	import { formatRelativeTime } from '../../lib/relative-time';
	import { listConnections, revokeConnection, type Connection } from '../../lib/connections';

	let connections = $state<Connection[]>([]);
	let loading = $state(true);
	let error = $state('');

	onMount(() => setTitle('Connections'));

	$effect(() => {
		if ($isInitialized && $isLoggedIn) load();
	});

	async function load() {
		loading = true;
		error = '';
		try {
			connections = await listConnections();
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			loading = false;
		}
	}

	async function revoke(connection: Connection) {
		const ok = await askConfirm({
			title: `Revoke ${connection.client_name}?`,
			body: `${connection.client_name} on ${connection.redirect_host} is signed out at once and can no longer act as you.`,
			confirmLabel: 'Revoke'
		});
		if (!ok) return;
		try {
			connections = await revokeConnection(connections, connection.id);
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
	}

	const DAY = 86400e3;
	const soon = (expires: string) => new Date(expires).getTime() - $time.getTime() < 3 * DAY;
</script>

<div class="ph">
	<div class="ph-row"><h1>Connections</h1></div>
	<p class="lead">
		Apps and command-line clients you allowed to act as you. Revoking one signs it out at once.
	</p>
</div>

{#if error}
	<div class="banner err">
		<Icon icon="ph:warning-circle" class="ic" aria-hidden="true" />
		<div class="txt"><b>Could not load or change connections</b><span>{error}</span></div>
	</div>
{/if}

<section class="sec">
	{#if loading}
		<span class="meta">Loading connections…</span>
	{:else if connections.length === 0 && !error}
		<div class="empty">
			<div class="sq">
				<Icon icon="ph:plugs-connected" class="ic lg" aria-hidden="true" />
			</div>
			<h2>No connections</h2>
			<p>Sign in with scottyctl or connect an MCP client to see it here.</p>
		</div>
	{:else}
		<div class="tw">
			<table>
				<thead>
					<tr>
						<th>Client</th>
						<th>Redirect host</th>
						<th>Scopes</th>
						<th class="num">Last used</th>
						<th class="num">Expires</th>
						<th><span class="sr-only">Action</span></th>
					</tr>
				</thead>
				<tbody>
					{#each connections as connection (connection.id)}
						<tr>
							<td
								><b>{connection.client_name}</b>
								<span class="tag">{connection.kind}</span></td
							>
							<td class="mono">{connection.redirect_host}</td>
							<td class="mono">{connection.scope}</td>
							<td class="num mono"
								>{connection.last_used_at
									? formatRelativeTime($time, connection.last_used_at)
									: 'never'}</td
							>
							<td class="num">
								{#if soon(connection.expires_at)}
									<span class="st warn"
										><Icon
											icon="ph:warning"
											class="ic sm"
											aria-hidden="true"
										/><span class="w"
											>{formatRelativeTime(
												$time,
												connection.expires_at
											)}</span
										></span
									>
								{:else}
									<span class="mono">{connection.expires_at.slice(0, 10)}</span>
								{/if}
							</td>
							<td class="num">
								<button class="btn outline sm" onclick={() => revoke(connection)}
									>Revoke</button
								>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{/if}
</section>
