<script>
	import logo from '$lib/assets/scotty.svg';
	import { onMount } from 'svelte';
	import { setTitle } from '../../stores/titleStore';
	import { authService } from '../../lib/authService';
	import { goto } from '$app/navigation';

	let password = '';
	let loading = true;
	let authMode = 'bearer';
	let message = '';
	let error = '';

	onMount(async () => {
		setTitle('Login');
		await checkAuthMode();
	});

	async function checkAuthMode() {
		try {
			const result = await authService.checkAuthMode();
			authMode = result.authMode;
			message = result.message || '';

			// Handle dev mode - redirect directly to dashboard
			if (authMode === 'dev') {
				await goto('/dashboard');
				return;
			}

			// Check if already authenticated and redirect
			if (authService.isAuthenticated()) {
				await goto('/dashboard');
				return;
			}

		} catch (err) {
			console.warn('Failed to check auth mode:', err);
			authMode = 'bearer'; // fallback
		}
		loading = false;
	}

	async function login() {
		if (authMode === 'oauth') {
			// Redirect to OAuth flow
			authService.initiateOAuthFlow();
			return;
		}

		// Handle bearer token login
		loading = true;
		error = '';

		try {
			const result = await authService.loginWithPassword(password);

			if (result.success) {
				if (result.redirectUrl) {
					window.location.href = result.redirectUrl;
				} else {
					await goto('/dashboard');
				}
			} else {
				error = result.error || 'Login failed';
			}
		} catch (err) {
			error = 'An unexpected error occurred';
			console.error('Login error:', err);
		}

		loading = false;
	}
</script>

{#if loading}
	<div class="card bg-base-100 w-96 shadow-md mx-auto">
		<div class="card-body">
			<div class="flex justify-center">
				<span class="loading loading-spinner loading-lg"></span>
			</div>
			<p class="text-center">Checking authentication...</p>
		</div>
	</div>
{:else}
	<div class="card bg-base-100 w-96 shadow-md mx-auto">
		<figure class="bg-gray-100 max-h-48">
			<img src={logo} alt="Scotty Logo" />
		</figure>
		<div class="card-body">
			<form on:submit|preventDefault={login}>
				<div class="card-body">
					<h2 class="card-title">
						{#if authMode === 'oauth'}
							OAuth Authentication
						{:else}
							Please log in
						{/if}
					</h2>

					{#if error}
						<div class="alert alert-error mb-4">
							<span>{error}</span>
						</div>
					{/if}

					{#if message}
						<p class="text-sm text-gray-600">{message}</p>
					{:else if authMode === 'oauth'}
						<p>You'll be redirected to your OIDC provider for authentication</p>
					{:else}
						<p>Please provide the password for this installation</p>
					{/if}

					{#if authMode === 'bearer'}
						<div class="my-4">
							<input
								class="input input-bordered w-full max-w-xs"
								type="password"
								bind:value={password}
								placeholder="Enter your password"
								required
								disabled={loading}
							/>
						</div>
					{/if}
				</div>
				<div class="card-actions justify-end">
					<button type="submit" class="btn btn-primary" disabled={loading}>
						{#if loading}
							<span class="loading loading-spinner loading-sm"></span>
						{/if}
						{#if authMode === 'oauth'}
							Continue with OAuth
						{:else}
							Login
						{/if}
					</button>
				</div>
			</form>
		</div>
	</div>
{/if}