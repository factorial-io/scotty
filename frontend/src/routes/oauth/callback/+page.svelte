<script lang="ts">
	import { onMount } from 'svelte';
	import Icon from '@iconify/svelte';
	import AuthFrame from '../../../components/auth-frame.svelte';
	import { page } from '$app/state';
	import { authService } from '../../../lib/authService';
	import { redirectAfterLogin } from '../../../lib/landingResume';

	let loading = true;
	let error: string | null = null;

	onMount(async () => {
		await handleOAuthCallback();
	});

	async function handleOAuthCallback() {
		try {
			const urlParams = page.url.searchParams;

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

				// If we came from the landing page flow, return there so the
				// app is started; otherwise go to the dashboard.
				await redirectAfterLogin();
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
	<title>Signing in | Scotty</title>
</svelte:head>

<AuthFrame center={loading}>
	{#if loading}
		<Icon icon="ph:spinner" class="ic lg spin" aria-hidden="true" />
		<h1>Completing sign-in</h1>
		<p class="sub2">Please wait while Scotty verifies your sign-in.</p>
	{:else if error}
		<h1>Sign-in failed</h1>
		<div class="banner err">
			<Icon icon="ph:warning-circle" class="ic" aria-hidden="true" />
			<div class="txt">
				<b>Your identity provider did not confirm the sign-in</b><span>{error}</span>
			</div>
		</div>
		<div class="acts">
			<button class="btn filled primary" on:click={handleRetry}>Try again</button>
		</div>
	{/if}
</AuthFrame>
