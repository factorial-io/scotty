<script lang="ts">
	import AppStatusPill from '../../../components/app-status-pill.svelte';
	import AuthFrame from '../../../components/auth-frame.svelte';
	import Icon from '@iconify/svelte';
	import { onMount, onDestroy } from 'svelte';
	import { resolve } from '$app/paths';
	import { goto } from '$app/navigation';
	import { isAuthenticated, authMode } from '../../../stores/sessionStore';
	import { authService } from '../../../lib/authService';
	import { storePendingLandingStart } from '../../../lib/landingResume';
	import { apps, runApp, updateAppInfo } from '../../../stores/appsStore';
	import { tasks } from '../../../stores/tasksStore';
	import {
		getTaskOutput,
		initializeTaskOutput,
		setTaskOutputSubscribed
	} from '../../../stores/taskOutputStore';
	import { webSocketStore } from '../../../stores/webSocketStore';
	import UnifiedOutput from '../../../components/unified-output.svelte';
	import TaskProgress from '../../../components/task-progress.svelte';
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

	$: taskOutput = taskId ? getTaskOutput(taskId) : null;

	// The store copy follows AppInfoUpdated, so an operation started elsewhere shows up here.
	$: app = $apps.find((a) => a.name === data.appName) ?? appInfo;
	$: operation = app?.operation ?? null;

	// The task store is kept current by the WebSocket, or by the polling
	// fallback in runApp() without one.
	$: taskState = taskId ? $tasks[taskId]?.state : undefined;
	$: if (state === 'starting' && taskState === 'Finished') {
		state = 'ready';
		startRedirectCountdown();
	}
	$: if (state === 'starting' && taskState === 'Failed') {
		state = 'error';
		appError = 'App failed to start. Check the output below for details.';
	}

	// An operation started elsewhere brought the app up.
	$: if (state === 'idle' && app && !operation && app.status === 'Running' && safeReturnUrl) {
		state = 'ready';
		startRedirectCountdown(safeReturnUrl);
	}

	// Request task output streaming when authenticated and task is active
	$: {
		if ($isAuthenticated && taskId && !taskOutputRequested) {
			webSocketStore.requestTaskOutputStream(taskId, true);
			taskOutputRequested = true;
		}
	}

	onMount(() => {
		setTitle(`Start: ${data.appName}`);
	});

	// The session may still be initializing at mount, so wait for it.
	let sessionReady = false;
	$: if ($isAuthenticated && !sessionReady) {
		sessionReady = true;
		onAuthenticated();
	}

	async function onAuthenticated() {
		// Always load app info first — this populates appInfo which is
		// needed for return_url domain validation.
		await loadAppInfo();
		// If app was already running, loadAppInfo triggers redirect — stop here.
		if (state === 'ready') return;

		// If autostart param is set, start immediately. The OAuth callback
		// redirects here with ?autostart=true after login. A running operation
		// is followed instead.
		if (data.autostart && state === 'idle' && !appInfo?.operation) {
			await startApp();
		}
	}

	onDestroy(() => {
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
			storePendingLandingStart({
				appName: data.appName,
				returnUrl: data.returnUrl
			});

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
			// Collected so the output can be shown if the start fails.
			initializeTaskOutput(id);
		} catch (err) {
			state = 'error';
			appError = err instanceof Error ? err.message : 'Failed to start app';
		}
	}

	function startRedirectCountdown(url?: string) {
		const redirectTo = url ?? safeReturnUrl;
		if (!redirectTo || countdownInterval) return;
		redirectCountdown = 3;
		countdownInterval = setInterval(() => {
			redirectCountdown -= 1;
			if (redirectCountdown <= 0) {
				if (countdownInterval) clearInterval(countdownInterval);
				countdownInterval = null;
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

	$: safeReturnUrl = isValidReturnUrl(data.returnUrl, app) ? data.returnUrl : null;
</script>

<svelte:head>
	<title>Start {data.appName} - Scotty</title>
</svelte:head>

<AuthFrame center={!(appError && state === 'error')}>
	{#if appError && state === 'error'}
		<h1>Failed to start {data.appName}</h1>
		<div class="banner err">
			<Icon icon="ph:warning-circle" class="ic" aria-hidden="true" />
			<div class="txt"><b>The start did not finish</b><span>{appError}</span></div>
		</div>
		<div class="acts">
			<a href={resolve('/dashboard')} class="btn outline">Go to dashboard</a>
			<button class="btn filled primary" on:click={handleRetry}>Try again</button>
		</div>
		{#if taskId && taskOutput && $taskOutput}
			<UnifiedOutput
				lines={$taskOutput.lines}
				heading=""
				loading={$taskOutput.loading}
				maxHeight="400px"
			/>
		{/if}
	{:else if state === 'ready'}
		<div class="ok-mark" style="color:var(--color-status-success)">
			<Icon icon="ph:check-circle" class="ic lg" aria-hidden="true" />
		</div>
		<h1>{data.appName} is ready</h1>
		{#if safeReturnUrl}
			<p class="sub2">Redirecting in {redirectCountdown}…</p>
			<div class="acts">
				<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
				<a href={safeReturnUrl} class="btn filled primary">Go to the app</a>
			</div>
		{:else}
			<p class="sub2"><b>{data.appName}</b> is now running.</p>
			<div class="acts">
				<a
					href={resolve('/dashboard/[slug]', { slug: data.appName })}
					class="btn filled primary">View in dashboard</a
				>
			</div>
		{/if}
	{:else if state === 'starting'}
		<Icon icon="ph:spinner" class="ic lg spin" aria-hidden="true" />
		<h1>Starting {data.appName}</h1>
		<p class="sub2">Please wait while the app is being started.</p>
		{#if taskId}<div class="progress-center"><TaskProgress {taskId} /></div>{/if}
	{:else}
		<h1>{data.appName}</h1>
		<p class="sub2">This app is currently not running.</p>
		{#if data.returnUrl}<p class="mono">{data.returnUrl}</p>{/if}
		{#if app}<AppStatusPill status={app.status} />{/if}
		{#if operation}
			<p class="meta">
				<Icon icon="ph:spinner" class="ic sm spin" aria-hidden="true" />
				{operation.operation} is running
			</p>
			{#if operation.task_id}
				<div class="progress-center"><TaskProgress taskId={operation.task_id} /></div>
			{/if}
		{/if}
		<div class="acts">
			<button class="btn filled primary" on:click={handleStart} disabled={!!operation}>
				<Icon icon="ph:play" class="ic sm" aria-hidden="true" />Start app
			</button>
		</div>
		{#if !$isAuthenticated}
			<p class="meta">You will be asked to sign in first.</p>
		{/if}
	{/if}
</AuthFrame>
