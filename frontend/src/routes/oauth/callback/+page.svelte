<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';
	import type { TokenResponse, OAuthErrorResponse } from '../../../types';
	import { authStore } from '../../../stores/userStore';

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

			// Get session ID from the new OAuth flow
			const sessionId = urlParams.get('session_id');

			if (!sessionId) {
				error = 'Missing session ID from OAuth callback';
				loading = false;
				return;
			}

			// Exchange session for token using the new endpoint
			const response = await fetch('/oauth/exchange', {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json'
				},
				body: JSON.stringify({
					session_id: sessionId
				})
			});

			if (!response.ok) {
				const errorData: OAuthErrorResponse = await response.json();
				error = `Authentication failed: ${errorData.error_description || errorData.error}`;
				loading = false;
				return;
			}

			const tokenData: TokenResponse = await response.json();

			const userInfo = {
				id: tokenData.user_id,
				name: tokenData.user_name,
				email: tokenData.user_email,
				picture: tokenData.user_picture
			};

			// Store token and user info in localStorage
			localStorage.setItem('oauth_token', tokenData.access_token);
			localStorage.setItem('user_info', JSON.stringify(userInfo));

			// Clear any old bearer tokens
			localStorage.removeItem('token');

			// Update the reactive store immediately
			authStore.setUserInfo(userInfo);

			// Load permissions
			const { loadUserPermissions } = await import('../../../stores/permissionStore');
			await loadUserPermissions();

			// Redirect to dashboard
			await goto('/dashboard');
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
