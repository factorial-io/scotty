<script lang="ts">
	import { onMount } from 'svelte';
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

<div class="card bg-base-100 w-[32rem] max-w-full shadow-md mx-auto">
	<div class="card-body">
		<h2 class="card-title">Connect an app to Scotty</h2>

		{#if consent.kind === 'loading' || consent.kind === 'redirect'}
			<span class="loading loading-spinner"></span>
		{:else if consent.kind === 'invalid'}
			<div class="alert alert-error">
				<span
					>This request is invalid or has expired; start the connection again from the
					app.</span
				>
			</div>
		{:else if consent.kind === 'error'}
			<div class="alert alert-error">
				<span>{consent.message}</span>
			</div>
		{:else}
			{@const request = consent.request}
			<div>
				<p class="text-sm text-gray-500">Access will be sent to</p>
				<p class="text-2xl font-bold font-mono break-all">{request.redirect_host}</p>
				{#if request.loopback_only}
					<p class="text-sm mt-1">
						The app receives the authorization only on this computer.
					</p>
				{/if}
			</div>
			<p>
				The app calls itself <strong>{request.client_name}</strong>
				<span class="text-sm text-gray-500">(self-declared, not verified)</span>. It wants
				to act as <strong>{$userInfo?.email || 'you'}</strong>.
			</p>
			<fieldset class="mt-2">
				<legend class="text-sm mb-2">Allow it to:</legend>
				{#each request.scopes as scope (scope)}
					<label class="label cursor-pointer justify-start gap-3">
						<input
							type="checkbox"
							class="checkbox checkbox-sm"
							bind:checked={ticked[scope]}
							disabled={busy}
						/>
						<span>{describeScope(scope)}</span>
						<span class="text-xs font-mono text-gray-500">{scope}</span>
					</label>
				{/each}
			</fieldset>
			<p class="text-sm text-gray-500">
				Only approve if you started this connection yourself and recognize the host.
			</p>
			<div class="card-actions justify-end mt-4">
				<button class="btn" onclick={() => decide(false)} disabled={busy}>Deny</button>
				<button
					class="btn btn-primary"
					onclick={() => decide(true)}
					disabled={busy || !approvable}
				>
					Approve
				</button>
			</div>
		{/if}
	</div>
</div>
