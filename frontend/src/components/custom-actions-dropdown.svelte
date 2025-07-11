<script lang="ts">
	import { onMount } from 'svelte';
	import { apiCall } from '$lib';
	import type { App } from '../types';
	import { goto } from '$app/navigation';

	export let app: App;

	let customActions: unknown[] = [];
	let isLoading = true;
	let currentTaskId: string | null = null;
	let currentAction: string | null = null;

	onMount(async () => {
		if (app.settings?.app_blueprint) {
			try {
				// Fetch all blueprints and filter for the one we need
				const result = await apiCall('blueprints');
				if (result && result.blueprints && result.blueprints[app.settings.app_blueprint]) {
					const blueprint = result.blueprints[app.settings.app_blueprint];
					// Filter for custom actions only
					// The API returns action names as "Custom(action_name)"
					// We need to extract the actual action name
					customActions = Object.keys(blueprint.actions || {})
						.filter((key) => !key.startsWith('post_'))
						.map((key) => ({
							name: key,
							description: blueprint.actions[key].description
						}));
				} else {
					console.warn(
						`Blueprint ${app.settings.app_blueprint} not found or has no actions`
					);
				}
			} catch (err) {
				console.error('Error loading blueprints:', err);
			} finally {
				isLoading = false;
			}
		} else {
			isLoading = false;
		}
	});

	async function triggerCustomAction(actionName: string) {
		if (currentTaskId !== null || !isSupported()) return;

		try {
			currentAction = actionName;
			const result = await apiCall(`apps/${app.name}/actions`, {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json'
				},
				body: JSON.stringify({ action_name: actionName })
			});

			// RunningAppContext contains a task field with the task details
			if (result && result.task && result.task.id) {
				currentTaskId = result.task.id;
				goto(`/tasks/${currentTaskId}`);
			} else {
				console.error('Unexpected API response:', result);
				throw new Error('Invalid response from server');
			}
		} catch (err) {
			console.error('Error triggering custom action:', err);
			currentAction = null;
		}
	}

	// Check if the app is in a supported state for custom actions
	function isSupported() {
		return app.status === 'Running' && app.settings?.app_blueprint;
	}
</script>

{#if isLoading}
	<div class="btn btn-sm join-item loading">Loading actions...</div>
{:else if customActions.length > 0}
	<div class="dropdown">
		<div
			tabindex="0"
			role="button"
			class="btn btn-sm join-item {currentTaskId !== null || !isSupported()
				? 'btn-disabled'
				: ''}"
		>
			{#if currentAction}
				<span class="loading loading-spinner loading-xs mr-1"></span>
				Running {currentAction}...
			{:else}
				Custom Actions
			{/if}
		</div>
		<ul
			tabindex="0"
			class="dropdown-content menu bg-base-100 rounded-box z-1 w-52 p-2 shadow-sm"
		>
			{#each customActions as action (action)}
				<li>
					<button
						on:click={() => triggerCustomAction(action.name)}
						data-tip={action.description}
						class="tooltip tooltip-right"
						disabled={currentTaskId !== null || !isSupported()}
					>
						{action.name}
					</button>
				</li>
			{/each}
		</ul>
	</div>
{:else if !isLoading && app.settings?.app_blueprint && isSupported()}
	<div
		class="btn btn-sm join-item btn-disabled tooltip tooltip-bottom"
		data-tip="No custom actions defined in blueprint"
	>
		Custom Actions
	</div>
{/if}
