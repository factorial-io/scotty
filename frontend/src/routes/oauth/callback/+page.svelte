<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/stores';
	import { authService } from '../../../lib/authService';

	let loading = true;
	let error: string | null = null;

	onMount(async () => {
		await handleOAuthCallback();
	});

	async function handleOAuthCallback() {
		try {
			const urlParams = $page.url.searchParams;

			// Check for OAuth error
			const oauthError = urlParams.get('error');
			if (oauthError) {
				const errorDescription =
					urlParams.get('error_description') || 'Unknown OAuth error';
				error = `OAuth Error: ${oauthError} - ${errorDescription}`;
				loading = false;
				return;
			}

			// Get session ID from the OAuth flow
			const sessionId = urlParams.get('session_id');

			if (!sessionId) {
				error = 'Missing session ID from OAuth callback';
				loading = false;
				return;
			}

			// Handle OAuth callback using AuthService
			const result = await authService.handleOAuthCallback(sessionId);

			if (result.success) {
				// Load permissions
				const { loadUserPermissions } = await import('../../../stores/permissionStore');
				await loadUserPermissions();

				// Check if we came from the landing page flow.
				// The sessionStorage key is read-then-removed here so that the
				// landing page receives the autostart=true query param instead of
				// re-reading from sessionStorage. This avoids a stale entry if the
				// user later visits the landing page for a different app.
				const pendingStart = sessionStorage.getItem('scotty_landing_pending_start');
				if (pendingStart) {
					try {
						const pending = JSON.parse(pendingStart);
						sessionStorage.removeItem('scotty_landing_pending_start');
						// eslint-disable-next-line svelte/prefer-svelte-reactivity -- non-reactive context
						const params = new URLSearchParams({ autostart: 'true' });
						if (pending.returnUrl) {
							params.set('return_url', pending.returnUrl);
						}
						await goto(
							resolve(`/landing/${encodeURIComponent(pending.appName)}?${params}`)
						);
						return;
					} catch {
						sessionStorage.removeItem('scotty_landing_pending_start');
					}
				}

				// Default: redirect to dashboard
				await goto(resolve('/dashboard'));
			} else {
				error = result.error || 'Authentication failed';
				loading = false;
			}
		} catch (err) {
			console.error('OAuth callback error:', err);
			error = err instanceof Error ? err.message : 'An unexpected error occurred';
			loading = false;
		}
	}

	function handleRetry() {
		window.location.href = '/login';
	}
</script>

<svelte:head>
	<title>OAuth Callback - Scotty</title>
</svelte:head>

{#if loading}
	<div class="card bg-base-100 w-96 shadow-md mx-auto">
		<div class="card-body">
			<div class="flex justify-center">
				<span class="loading loading-spinner loading-lg"></span>
			</div>
			<p class="text-center">Completing authentication...</p>
			<p class="text-center text-sm text-gray-500">
				Please wait while we verify your credentials
			</p>
		</div>
	</div>
{:else if error}
	<div class="card bg-base-100 w-96 shadow-md mx-auto">
		<div class="card-body">
			<h2 class="card-title text-error">Authentication Failed</h2>
			<p class="text-sm">{error}</p>
			<div class="card-actions justify-end mt-4">
				<button class="btn btn-primary" on:click={handleRetry}> Try Again </button>
			</div>
		</div>
	</div>
{/if}
