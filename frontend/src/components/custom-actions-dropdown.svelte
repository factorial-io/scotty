<script lang="ts">
	import { authenticatedApiCall } from '#lib';
	import type { App, BlueprintsResponse, CustomAction } from '../types';
	import { runCustomAction } from '../stores/appsStore';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { sessionStore } from '../stores/sessionStore';

	import { showError } from '../stores/errorStore';

	export let app: App;
	export let canManage: boolean = false;

	let customActions: CustomAction[] = [];
	let isLoading = true;
	let hasFetched = false;
	let lastAppKey = `${app.name}-${app.settings?.app_blueprint || 'none'}`;

	// Export a reactive value that indicates if actions are available
	export let hasActions: boolean = false;
	// Set between the click and the server's response.
	let currentAction: string | null = null;

	// While the app is busy only read actions may run; they take no lock.
	$: busy = currentAction !== null || !!app.operation;
	function isAvailable(action: CustomAction, busy: boolean): boolean {
		return !busy || (currentAction === null && action.permission === 'action_read');
	}

	// Reactive statement: Fetch blueprints when authentication is ready and canManage is true
	$: {
		const shouldFetch =
			$sessionStore.isAuthenticated &&
			$sessionStore.isInitialized &&
			canManage &&
			app.settings?.app_blueprint &&
			!hasFetched;

		if (shouldFetch) {
			fetchBlueprints();
		}
	}

	async function fetchBlueprints() {
		// Prevent multiple fetches
		if (hasFetched) return;
		hasFetched = true;
		isLoading = true;

		try {
			// Check if app settings exist
			if (!app.settings) {
				console.warn('App settings not available');
				return;
			}

			// Fetch all blueprints and filter for the one we need
			const result = (await authenticatedApiCall('blueprints')) as BlueprintsResponse;
			if (result && result.blueprints && result.blueprints[app.settings.app_blueprint]) {
				const blueprint = result.blueprints[app.settings.app_blueprint];
				// Filter for custom actions only
				customActions = Object.keys(blueprint.actions || {})
					.filter((key) => !key.startsWith('post_'))
					.map((key) => ({
						name: key,
						description: blueprint.actions[key].description,
						permission: blueprint.actions[key].permission
					}));
			} else {
				console.warn(`Blueprint ${app.settings.app_blueprint} not found or has no actions`);
			}
		} catch (err) {
			console.error('Error loading blueprints:', err);
			// Reset hasFetched to allow retry
			hasFetched = false;
		} finally {
			isLoading = false;
		}
	}

	// Reset hasFetched when app name or blueprint changes
	$: if (app.name || app.settings?.app_blueprint) {
		// Only reset if the app identity actually changed
		const currentAppKey = `${app.name}-${app.settings?.app_blueprint || 'none'}`;
		if (currentAppKey !== lastAppKey) {
			lastAppKey = currentAppKey;
			hasFetched = false;
			customActions = [];
			isLoading = true;
		}
	}

	async function triggerCustomAction(actionName: string) {
		const action = customActions.find((a) => a.name === actionName);
		if (!action || !isAvailable(action, busy) || !isSupported() || !canManage) return;

		try {
			currentAction = actionName;
			const taskId = await runCustomAction(app.name, actionName);
			goto(resolve('/tasks/[slug]', { slug: taskId }));
		} catch (err) {
			console.error('Error triggering custom action:', err);
			showError(`${actionName} failed`, err);
		} finally {
			currentAction = null;
		}
	}

	// Check if the app is in a supported state for custom actions
	function isSupported(): boolean {
		return app.status === 'Running' && !!app.settings?.app_blueprint;
	}

	// Update the exported hasActions variable reactively
	$: hasActions = !isLoading && customActions.length > 0 && canManage && isSupported();
</script>

{#if hasActions}
	<div class="dropdown dropdown-end">
		<div
			tabindex="0"
			role="button"
			class="btn btn-sm {currentAction !== null || !canManage || !isSupported()
				? 'btn-disabled'
				: ''}"
		>
			{#if currentAction}
				<span class="loading loading-spinner loading-xs mr-1"></span>
				Running {currentAction}...
			{:else if app.operation}
				<span class="loading loading-spinner loading-xs mr-1"></span>
				Custom Actions
			{:else}
				Custom Actions
			{/if}
		</div>
		<ul
			role="menu"
			tabindex="0"
			class="dropdown-content menu bg-base-100 rounded-box z-10 w-52 p-2 shadow-sm"
		>
			{#each customActions as action (action)}
				<li>
					<button
						on:click={() => triggerCustomAction(action.name)}
						data-tip={action.description}
						class="tooltip tooltip-right"
						disabled={!isAvailable(action, busy) || !canManage || !isSupported()}
					>
						{action.name}
					</button>
				</li>
			{/each}
		</ul>
	</div>
{/if}
