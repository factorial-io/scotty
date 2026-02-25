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

		// Check if we're returning from OAuth with a pending start
		const pendingStart = sessionStorage.getItem('scotty_landing_pending_start');
		if (pendingStart) {
			try {
				const pending = JSON.parse(pendingStart);
				if (pending.appName === data.appName) {
					sessionStorage.removeItem('scotty_landing_pending_start');
					// Auto-trigger start after OAuth return
					if ($isAuthenticated) {
						await startApp();
						return;
					}
				}
			} catch {
				sessionStorage.removeItem('scotty_landing_pending_start');
			}
		}

		// If autostart param is set and user is authenticated, start immediately
		if (data.autostart && $isAuthenticated) {
			await startApp();
			return;
		}

		// Try to load app info (requires auth)
		if ($isAuthenticated) {
			await loadAppInfo();
		}
	});

	onDestroy(() => {
		if (taskId) {
			webSocketStore.stopTaskOutputStream(taskId);
			setTaskOutputSubscribed(taskId, false);
		}
		if (countdownInterval) {
			clearInterval(countdownInterval);
		}
	});

	async function loadAppInfo() {
		try {
			const result = await updateAppInfo(data.appName);
			if ('error' in result && result.error) {
				appError = 'App not found';
			} else {
				appInfo = result as App;
				// If the app is already running, redirect immediately
				if (appInfo.status === 'Running' && data.returnUrl) {
					state = 'ready';
					startRedirectCountdown();
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

		try {
			const id = await runApp(data.appName);
			taskId = id;
			initializeTaskOutput(id);

			// Monitor task completion
			monitorTask(id, async (result) => {
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

	function startRedirectCountdown() {
		if (!data.returnUrl) return;
		redirectCountdown = 3;
		countdownInterval = setInterval(() => {
			redirectCountdown -= 1;
			if (redirectCountdown <= 0) {
				if (countdownInterval) clearInterval(countdownInterval);
				window.location.href = data.returnUrl!;
			}
		}, 1000);
	}

	function handleRetry() {
		state = 'idle';
		appError = null;
		taskId = null;
		taskOutputRequested = false;
	}
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
				{#if data.returnUrl}
					<p class="text-gray-500 mt-2">
						Redirecting in {redirectCountdown}...
					</p>
					<div class="card-actions mt-4">
						<a href={data.returnUrl} class="btn btn-primary">Go to App</a>
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
					<p class="text-sm text-gray-400 mt-4">
						You will be asked to log in first.
					</p>
				{/if}
			</div>
		</div>
	{/if}
</div>
