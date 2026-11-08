<script lang="ts">
	import { onMount } from 'svelte';
	import { ApiError } from '$lib';
	import PageHeader from '../../components/page-header.svelte';
	import { setTitle } from '../../stores/titleStore';
	import { isInitialized, isLoggedIn } from '../../stores/userStore';
	import { hasPermission, permissions } from '../../stores/permissionStore';
	import { listConnections, revokeConnection, type Connection } from '../../lib/connections';

	let connections = $state<Connection[]>([]);
	let all = $state(false);
	let loading = $state(true);
	let error = $state('');
	let forbidden = $state(false);

	// Listing everyone's connections needs AdminWrite; the server enforces it,
	// a 403 hides the toggle as well.
	let isAdmin = $derived($permissions && !forbidden && hasPermission('_global', 'admin_write'));

	onMount(() => setTitle('Connections'));

	$effect(() => {
		if ($isInitialized && $isLoggedIn) load(all);
	});

	async function load(everyone: boolean) {
		loading = true;
		error = '';
		try {
			connections = await listConnections(everyone);
		} catch (e) {
			if (everyone && e instanceof ApiError && e.status === 403) {
				forbidden = true;
				all = false;
				return;
			}
			error = e instanceof Error ? e.message : String(e);
		} finally {
			loading = false;
		}
	}

	async function revoke(connection: Connection) {
		const who = all ? ` of ${connection.subject}` : '';
		if (
			!confirm(
				`Revoke the connection "${connection.client_name}"${who}? It stops working immediately.`
			)
		)
			return;
		try {
			connections = await revokeConnection(connections, connection.id);
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
	}

	const date = (value: string | null) => (value ? new Date(value).toLocaleString() : 'never');
</script>

<PageHeader>
	<h2 class="card-title" slot="header">Connections</h2>
	<div slot="meta">
		{#if isAdmin}
			<label class="label cursor-pointer gap-2">
				<input type="checkbox" class="toggle toggle-sm" bind:checked={all} />
				<span>All users</span>
			</label>
		{/if}
	</div>
</PageHeader>

<p class="text-sm text-gray-500 mb-4">
	Apps and command-line clients you allowed to act as you. Revoking one signs it out at once.
</p>

{#if error}
	<div class="alert alert-error mb-4"><span>{error}</span></div>
{/if}

{#if loading}
	<span class="loading loading-spinner"></span>
{:else if connections.length === 0}
	<p>No connections.</p>
{:else}
	<div class="overflow-x-auto">
		<table class="table table-sm">
			<thead>
				<tr>
					<th>Client</th>
					{#if all}<th>User</th>{/if}
					<th>Redirect host</th>
					<th>Scopes</th>
					<th>Created</th>
					<th>Last used</th>
					<th>Expires</th>
					<th></th>
				</tr>
			</thead>
			<tbody>
				{#each connections as connection (connection.id)}
					<tr>
						<td>
							{connection.client_name}
							<span class="badge badge-ghost badge-sm uppercase"
								>{connection.kind}</span
							>
						</td>
						{#if all}<td>{connection.subject}</td>{/if}
						<td class="font-mono">{connection.redirect_host}</td>
						<td class="font-mono text-xs">{connection.scope}</td>
						<td>{date(connection.created_at)}</td>
						<td>{date(connection.last_used_at)}</td>
						<td>{date(connection.expires_at)}</td>
						<td>
							<button class="btn btn-xs btn-error" onclick={() => revoke(connection)}
								>Revoke</button
							>
						</td>
					</tr>
				{/each}
			</tbody>
		</table>
	</div>
{/if}
