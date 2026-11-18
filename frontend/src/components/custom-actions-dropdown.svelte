<script lang="ts">
	import Icon from '@iconify/svelte';
	import { DropdownMenu } from 'bits-ui';
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
	<DropdownMenu.Root>
		<DropdownMenu.Trigger
			class="btn outline"
			disabled={currentAction !== null || !canManage || !isSupported()}
		>
			{#if currentAction || app.operation}
				<Icon icon="ph:spinner" class="ic sm spin" aria-hidden="true" />
			{:else}
				<Icon icon="ph:lightning" class="ic sm" aria-hidden="true" />
			{/if}
			{currentAction ? `Running ${currentAction}…` : 'Custom actions'}
			<Icon icon="ph:caret-down" class="ic sm" aria-hidden="true" />
		</DropdownMenu.Trigger>
		<DropdownMenu.Portal>
			<DropdownMenu.Content class="menu" align="end" sideOffset={6}>
				<div class="hd">Custom actions</div>
				{#each customActions as action (action.name)}
					<DropdownMenu.Item
						disabled={!isAvailable(action, busy) || !canManage || !isSupported()}
						onSelect={() => triggerCustomAction(action.name)}
						data-action={action.name}
					>
						<Icon icon="ph:terminal-window" class="ic sm" aria-hidden="true" />
						<span>{action.name}<span class="sub">{action.description}</span></span>
					</DropdownMenu.Item>
				{/each}
			</DropdownMenu.Content>
		</DropdownMenu.Portal>
	</DropdownMenu.Root>
{/if}
