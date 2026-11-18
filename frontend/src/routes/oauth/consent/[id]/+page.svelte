<script lang="ts">
	import { onMount } from 'svelte';
	import Icon from '@iconify/svelte';
	import AuthFrame from '../../../../components/auth-frame.svelte';
	import { page } from '$app/state';
	import { setTitle } from '../../../../stores/titleStore';
	import { isInitialized, isLoggedIn, userInfo } from '../../../../stores/userStore';
	import {
		canApprove,
		decideConsent,
		describeScope,
		loadConsent,
		selectedScopes,
		type ConsentState
	} from '../../../../lib/consent';

	let consent = $state<ConsentState>({ kind: 'loading' });
	let ticked = $state<Record<string, boolean>>({});
	let busy = $state(false);
	let started = false;

	let requested = $derived(consent.kind === 'pending' ? consent.request.scopes : []);
	let approvable = $derived(canApprove(requested, ticked));

	onMount(() => setTitle('Connect an app'));

	// Load once the session is known; the layout sends logged-out users to
	// login and back here.
	$effect(() => {
		if (!started && $isInitialized && $isLoggedIn) {
			started = true;
			load();
		}
	});

	async function load() {
		consent = await loadConsent(page.params.id ?? '');
		if (consent.kind === 'pending') {
			ticked = Object.fromEntries(consent.request.scopes.map((s) => [s, true]));
		}
	}

	async function decide(approve: boolean) {
		busy = true;
		const result = await decideConsent(
			page.params.id ?? '',
			approve ? selectedScopes(requested, ticked) : null
		);
		if (result.kind === 'redirect') {
			// The client's redirect URI is on another origin.
			window.location.assign(result.url);
			return;
		}
		consent = result;
		busy = false;
	}
</script>

<AuthFrame>
	<h1>Connect an app to Scotty</h1>
	{#if consent.kind === 'loading' || consent.kind === 'redirect'}
		<p class="meta">
			<Icon icon="ph:spinner" class="ic sm spin" aria-hidden="true" />
			{consent.kind === 'redirect' ? 'Sending you back to the app…' : 'Loading the request…'}
		</p>
	{:else if consent.kind === 'invalid'}
		<div class="banner err">
			<Icon icon="ph:warning-circle" class="ic" aria-hidden="true" />
			<div class="txt">
				<b>This request is invalid or has expired</b>
				<span>Start the connection again from the app.</span>
			</div>
		</div>
	{:else if consent.kind === 'error'}
		<div class="banner err">
			<Icon icon="ph:warning-circle" class="ic" aria-hidden="true" />
			<div class="txt"><b>Something went wrong</b><span>{consent.message}</span></div>
		</div>
	{:else}
		{@const request = consent.request}
		<div>
			<p class="meta">Access will be sent to</p>
			<p class="big-host">{request.redirect_host}</p>
			{#if request.loopback_only}
				<p class="sub2">The app receives the authorization only on this computer.</p>
			{/if}
		</div>
		<p>
			The app calls itself <b>{request.client_name}</b>
			<span class="sub2">(self-declared, not verified)</span>. It wants to act as
			<b>{$userInfo?.email || 'you'}</b>.
		</p>
		<fieldset class="scopes">
			<legend>Allow it to</legend>
			{#each request.scopes as scope (scope)}
				<label>
					<input type="checkbox" bind:checked={ticked[scope]} disabled={busy} />
					<span>{describeScope(scope)}<span class="mono">{scope}</span></span>
				</label>
			{/each}
		</fieldset>
		<p class="sub2">
			Only approve if you started this connection yourself and recognize the host.
		</p>
		<div class="acts">
			<button class="btn outline" onclick={() => decide(false)} disabled={busy}>Deny</button>
			<button
				class="btn filled primary"
				onclick={() => decide(true)}
				disabled={busy || !approvable}>Approve</button
			>
		</div>
	{/if}
</AuthFrame>
