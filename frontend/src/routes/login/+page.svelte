<script>
	import logo from '$lib/assets/scotty.svg';
	import { onMount } from 'svelte';
	import { setTitle } from '../../stores/titleStore';

	let password = '';
	let loading = true;
	let authMode = 'bearer';
	let oauthRedirectUrl = '/oauth2/start';
	let message = '';

	onMount(async () => {
		setTitle('Login');
		await checkAuthMode();
	});

	async function checkAuthMode() {
		try {
			const response = await fetch('/api/v1/login', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ password: '' }),
				credentials: 'include'
			});

			if (response.ok) {
				const result = await response.json();
				authMode = result.auth_mode || 'bearer';
				oauthRedirectUrl = result.redirect_url || '/oauth2/start';
				message = result.message || '';

				// Handle different auth modes
				if (authMode === 'dev') {
					// Development mode - redirect directly to dashboard
					window.location.href = '/dashboard';
					return;
				} else if (authMode === 'oauth' && result.status === 'redirect') {
					// OAuth mode - redirect to OAuth provider
					window.location.href = oauthRedirectUrl;
					return;
				}
			}
		} catch (error) {
			console.warn('Failed to check auth mode:', error);
			authMode = 'bearer'; // fallback
		}
		loading = false;
	}

	async function login() {
		if (authMode === 'oauth') {
			window.location.href = oauthRedirectUrl;
			return;
		}

		const response = await fetch('/api/v1/login', {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json'
			},
			body: JSON.stringify({ password }),
			credentials: 'include'
		});

		if (response.ok) {
			const result = await response.json();
			if (result.status === 'success') {
				if (result.token) {
					localStorage.setItem('token', result.token);
				}
				window.location.href = '/dashboard';
			} else if (result.status === 'redirect') {
				window.location.href = result.redirect_url;
			}
		} else {
			alert('Login failed');
		}
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
					
					{#if message}
						<p class="text-sm text-gray-600">{message}</p>
					{:else if authMode === 'oauth'}
						<p>You'll be redirected to GitLab for authentication</p>
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
							/>
						</div>
					{/if}
				</div>
				<div class="card-actions justify-end">
					<button type="submit" class="btn btn-primary">
						{#if authMode === 'oauth'}
							Continue to GitLab
						{:else}
							Login
						{/if}
					</button>
				</div>
			</form>
		</div>
	</div>
{/if}
