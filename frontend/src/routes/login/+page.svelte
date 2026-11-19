<script>
	import Icon from '@iconify/svelte';
	import AuthFrame from '../../components/auth-frame.svelte';
	import { onMount } from 'svelte';
	import { setTitle } from '../../stores/titleStore';
	import { authService } from '../../lib/authService';
	import { redirectAfterLogin } from '../../lib/landingResume';

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
				await redirectAfterLogin();
				return;
			}

			// Check if already authenticated and redirect
			if (authService.isAuthenticated()) {
				await redirectAfterLogin();
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
					await redirectAfterLogin();
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

<AuthFrame>
	{#if loading}
		<p class="meta">Checking how this server signs you in…</p>
	{:else}
		<h1>Sign in to Scotty</h1>
		{#if error}
			<div class="banner err">
				<Icon icon="ph:warning-circle" class="ic" aria-hidden="true" />
				<div class="txt"><b>Sign-in failed</b><span>{error}</span></div>
			</div>
		{/if}
		<p class="sub2">
			{#if message}
				{message}
			{:else if authMode === 'oauth'}
				You'll be sent to your company sign-in (OIDC) and come back here afterwards.
			{:else}
				Enter the access token for this installation.
			{/if}
		</p>
		<form on:submit|preventDefault={login} style="display:flex;flex-direction:column;gap:20px">
			{#if authMode === 'bearer'}
				<label class="field">
					<Icon icon="ph:key" class="ic" aria-hidden="true" />
					<input
						type="password"
						bind:value={password}
						placeholder="Access token"
						aria-label="Access token"
						required
						disabled={loading}
					/>
				</label>
			{/if}
			<div class="acts">
				<button type="submit" class="btn filled primary" disabled={loading}>
					{#if loading}
						<Icon icon="ph:spinner" class="ic sm spin" aria-hidden="true" />
					{:else}
						<Icon icon="ph:sign-in" class="ic sm" aria-hidden="true" />
					{/if}
					{authMode === 'oauth' ? 'Continue with OAuth' : 'Sign in'}
				</button>
			</div>
		</form>
	{/if}
</AuthFrame>
