<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { resolve } from '$app/paths';
	import { goto } from '$app/navigation';
	import { isAuthenticated, authMode } from '../../../stores/sessionStore';
	import { authService } from '../../../lib/authService';
	import { runApp, updateAppInfo } from '../../../stores/appsStore';
	import { monitorTask } from '../../../stores/tasksStore';
	import {
		getTaskOutput,
		initializeTaskOutput,
		setTaskOutputSubscribed
	} from '../../../stores/taskOutputStore';
	import { webSocketStore } from '../../../stores/webSocketStore';
	import UnifiedOutput from '../../../components/unified-output.svelte';
	import type { App } from '../../../types';
	import { setTitle } from '../../../stores/titleStore';

	export let data: { appName: string; returnUrl: string | null; autostart: boolean };

	type PageState = 'idle' | 'starting' | 'ready' | 'error';

	let state: PageState = 'idle';
	let appInfo: App | null = null;
	let appError: string | null = null;
	let taskId: string | null = null;
	let taskOutputRequested = false;
	let redirectCountdown = 3;
	let countdownInterval: ReturnType<typeof setInterval> | null = null;
	let destroyed = false;

	$: taskOutput = taskId ? getTaskOutput(taskId) : null;

	// Request task output streaming when authenticated and task is active
	$: {
		if ($isAuthenticated && taskId && !taskOutputRequested) {
			webSocketStore.requestTaskOutputStream(taskId, true);
			taskOutputRequested = true;
		}
	}

	onMount(async () => {
		setTitle(`Start: ${data.appName}`);

		// Always load app info first when authenticated — this populates
		// appInfo which is needed for return_url domain validation.
		if ($isAuthenticated) {
			await loadAppInfo();
			// If app was already running, loadAppInfo triggers redirect — stop here.
			if (state === 'ready') return;
		}

		// If autostart param is set and user is authenticated, start immediately.
		// The OAuth callback redirects here with ?autostart=true after login.
		if (data.autostart && $isAuthenticated) {
			await startApp();
			return;
		}
	});

	onDestroy(() => {
		destroyed = true;
		cleanupTask();
		if (countdownInterval) {
			clearInterval(countdownInterval);
		}
	});

	function cleanupTask() {
		if (taskId) {
			webSocketStore.stopTaskOutputStream(taskId);
			setTaskOutputSubscribed(taskId, false);
		}
	}

	async function loadAppInfo() {
		try {
			const result = await updateAppInfo(data.appName);
			if ('error' in result && result.error) {
				appError = 'App not found';
			} else {
				appInfo = result as App;
				// Evaluate inline instead of relying on $: reactive statement,
				// which wouldn't update until after this synchronous block.
				const validUrl = isValidReturnUrl(data.returnUrl, appInfo) ? data.returnUrl : null;
				if (appInfo.status === 'Running' && validUrl) {
					state = 'ready';
					startRedirectCountdown(validUrl);
				}
			}
		} catch {
			// Not critical -- user can still click Start
		}
	}

	async function handleStart() {
		if (!$isAuthenticated) {
			// Store intent to start after login
			sessionStorage.setItem(
				'scotty_landing_pending_start',
				JSON.stringify({
					appName: data.appName,
					returnUrl: data.returnUrl
				})
			);

			// Redirect to login -- after OAuth, user returns to /landing/[slug]
			if ($authMode === 'oauth') {
				authService.initiateOAuthFlow();
			} else {
				await goto(resolve('/login'));
			}
			return;
		}

		await startApp();
	}

	async function startApp() {
		state = 'starting';
		appError = null;

		// Ensure appInfo is loaded so safeReturnUrl can be validated after
		// the task completes. Without this, if loadAppInfo() failed earlier
		// (e.g. network issue), the ready-state redirect would silently
		// fall back to the dashboard link instead of the original URL.
		if (!appInfo && $isAuthenticated) {
			await loadAppInfo();
		}

		try {
			const id = await runApp(data.appName);
			taskId = id;
			initializeTaskOutput(id);

			// Monitor task completion. The callback may fire after the component
			// is destroyed (e.g. user navigates away), so guard with `destroyed`.
			monitorTask(id, async (result) => {
				if (destroyed) return;
				if (result.state === 'Finished') {
					state = 'ready';
					startRedirectCountdown();
				} else if (result.state === 'Failed') {
					state = 'error';
					appError = 'App failed to start. Check the output below for details.';
				}
			});
		} catch (err) {
			state = 'error';
			appError = err instanceof Error ? err.message : 'Failed to start app';
		}
	}

	function startRedirectCountdown(url?: string) {
		const redirectTo = url ?? safeReturnUrl;
		if (!redirectTo) return;
		redirectCountdown = 3;
		countdownInterval = setInterval(() => {
			redirectCountdown -= 1;
			if (redirectCountdown <= 0) {
				if (countdownInterval) clearInterval(countdownInterval);
				window.location.href = redirectTo;
			}
		}, 1000);
	}

	function handleRetry() {
		cleanupTask();
		state = 'idle';
		appError = null;
		taskId = null;
		taskOutputRequested = false;
	}

	/**
	 * Validate the return URL for safe redirect:
	 * - Must be http or https (rejects javascript:, data:, etc.)
	 * - If app info is loaded, hostname must match one of the app's known domains
	 *   (both runtime container domains and settings-based domains)
	 * - If app info is not loaded, reject the URL (require domain validation)
	 */
	function isValidReturnUrl(url: string | null, app: App | null): url is string {
		if (!url) return false;
		try {
			const parsed = new URL(url);
			if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') {
				return false;
			}
			// Require app info for domain validation — without it we cannot
			// verify the return URL belongs to the app, so reject it.
			if (!app) return false;

			const hostname = parsed.hostname.toLowerCase();

			// Runtime container domains (from running/previously-running state)
			const runtimeDomains = app.services.flatMap((s) =>
				s.domains.map((d) => d.toLowerCase())
			);

			// Settings-based domains: explicit + auto-generated {service}.{domain}
			// Auto-generated format must match Traefik label generation in
			// scotty/src/docker/loadbalancer/traefik.rs
			const settingsDomains = (app.settings?.public_services ?? []).flatMap((s) => {
				const explicit = s.domains.map((d) => d.toLowerCase());
				const autoGen = app.settings?.domain
					? [`${s.service}.${app.settings.domain}`.toLowerCase()]
					: [];
				return [...explicit, ...autoGen];
			});

			const appDomains = [...new Set([...runtimeDomains, ...settingsDomains])];

			// Reject if the app has no known domains or hostname doesn't match
			if (appDomains.length === 0 || !appDomains.includes(hostname)) {
				return false;
			}
			return true;
		} catch {
			return false;
		}
	}

	$: safeReturnUrl = isValidReturnUrl(data.returnUrl, appInfo) ? data.returnUrl : null;
</script>

<svelte:head>
	<title>Start {data.appName} - Scotty</title>
</svelte:head>

<div class="max-w-xl mx-auto mt-8">
	{#if appError && state === 'error'}
		<!-- ERROR STATE -->
		<div class="card bg-base-100 shadow-md">
			<div class="card-body items-center text-center">
				<h2 class="card-title text-error text-2xl">Failed to start</h2>
				<p class="text-gray-500 mt-2">{appError}</p>
				<div class="card-actions mt-6">
					<button class="btn btn-primary" on:click={handleRetry}>Try Again</button>
					<a href={resolve('/dashboard')} class="btn btn-ghost">Go to Dashboard</a>
				</div>
			</div>
		</div>

		{#if taskId && taskOutput && $taskOutput}
			<div class="mt-6">
				<h3 class="text-lg mb-2">Task Output</h3>
				<UnifiedOutput
					lines={$taskOutput.lines}
					heading=""
					loading={$taskOutput.loading}
					maxHeight="400px"
				/>
			</div>
		{/if}
	{:else if state === 'ready'}
		<!-- READY STATE -->
		<div class="card bg-base-100 shadow-md">
			<div class="card-body items-center text-center">
				<div class="text-success text-5xl mb-4">&#10003;</div>
				<h2 class="card-title text-2xl">App is ready!</h2>
				{#if safeReturnUrl}
					<p class="text-gray-500 mt-2">
						Redirecting in {redirectCountdown}...
					</p>
					<div class="card-actions mt-4">
						<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
						<a href={safeReturnUrl} class="btn btn-primary">Go to App</a>
					</div>
				{:else}
					<p class="text-gray-500 mt-2">
						<strong>{data.appName}</strong> is now running.
					</p>
					<div class="card-actions mt-4">
						<a href={resolve(`/dashboard/${data.appName}`)} class="btn btn-primary"
							>View in Dashboard</a
						>
					</div>
				{/if}
			</div>
		</div>
	{:else if state === 'starting'}
		<!-- STARTING STATE -->
		<div class="card bg-base-100 shadow-md">
			<div class="card-body items-center text-center">
				<span class="loading loading-spinner loading-lg"></span>
				<h2 class="card-title text-2xl mt-4">Starting {data.appName}...</h2>
				<p class="text-gray-500 mt-2">Please wait while the app is being started.</p>
			</div>
		</div>

		{#if taskId && taskOutput && $taskOutput}
			<div class="mt-6">
				<h3 class="text-lg mb-2">Progress</h3>
				<UnifiedOutput
					lines={$taskOutput.lines}
					heading=""
					loading={$taskOutput.loading}
					maxHeight="400px"
				/>
			</div>
		{/if}
	{:else}
		<!-- IDLE STATE -->
		<div class="card bg-base-100 shadow-md">
			<div class="card-body items-center text-center">
				<h2 class="card-title text-2xl">{data.appName}</h2>
				<p class="text-gray-500 mt-2">This app is currently not running.</p>
				{#if data.returnUrl}
					<p class="text-sm text-gray-400 mt-1">
						<span class="font-mono">{data.returnUrl}</span>
					</p>
				{/if}
				{#if appInfo}
					<div class="badge badge-ghost mt-2">{appInfo.status}</div>
				{/if}

				<div class="card-actions mt-6">
					<button class="btn btn-primary btn-lg" on:click={handleStart}>
						Start App
					</button>
				</div>

				{#if !$isAuthenticated}
					<p class="text-sm text-gray-400 mt-4">You will be asked to log in first.</p>
				{/if}
			</div>
		</div>
	{/if}
</div>
