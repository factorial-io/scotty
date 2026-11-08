<script lang="ts">
	import { onMount } from 'svelte';
	import { setTitle } from '../../stores/titleStore';
	import { isInitialized, isLoggedIn, userInfo } from '../../stores/userStore';
	import {
		codeFromUrl,
		decideDeviceCode,
		lookupDeviceCode,
		normalizeUserCode,
		scopeList,
		type DeviceState
	} from '../../lib/deviceAuth';

	let code = $state('');
	let prefilled = $state(false);
	let deviceState = $state<DeviceState>({ kind: 'idle' });
	let busy = $derived(deviceState.kind === 'loading');

	onMount(() => {
		setTitle('Authorize device');
		// Read in the browser: the page is prerendered, so the query is not known at build time.
		code = codeFromUrl(new URL(window.location.href));
		prefilled = code !== '';
	});

	// Look up a prefilled code once the session is known (the layout sends
	// logged-out users to login and back here).
	$effect(() => {
		if (prefilled && $isInitialized && $isLoggedIn && deviceState.kind === 'idle') {
			prefilled = false;
			lookup();
		}
	});

	async function lookup() {
		deviceState = { kind: 'loading' };
		deviceState = await lookupDeviceCode(code);
	}

	async function decide(approve: boolean) {
		deviceState = { kind: 'loading' };
		deviceState = await decideDeviceCode(code, approve);
	}

	function onsubmit(event: SubmitEvent) {
		event.preventDefault();
		code = normalizeUserCode(code);
		lookup();
	}
</script>

<div class="card bg-base-100 w-[28rem] max-w-full shadow-md mx-auto">
	<div class="card-body">
		<h2 class="card-title">Authorize a device</h2>

		{#if deviceState.kind === 'pending'}
			<p>
				<strong>{deviceState.request.client_name}</strong> wants to access Scotty as
				<strong>{$userInfo?.email || 'you'}</strong>.
			</p>
			<p class="text-sm text-gray-500">
				Code <span class="font-mono">{deviceState.request.user_code}</span>
			</p>
			{#if scopeList(deviceState.request.scope).length > 0}
				<div>
					<p class="text-sm">Requested access:</p>
					<ul class="list-disc list-inside text-sm">
						{#each scopeList(deviceState.request.scope) as scope (scope)}
							<li class="font-mono">{scope}</li>
						{/each}
					</ul>
				</div>
			{/if}
			<p class="text-sm text-gray-500">Only approve if you started this login yourself.</p>
			<div class="card-actions justify-end mt-4">
				<button class="btn" onclick={() => decide(false)}>Deny</button>
				<button class="btn btn-primary" onclick={() => decide(true)}>Approve</button>
			</div>
		{:else if deviceState.kind === 'approved'}
			<div class="alert alert-success">
				<span>Device approved. You can return to your terminal.</span>
			</div>
		{:else if deviceState.kind === 'denied'}
			<div class="alert alert-warning">
				<span>Access denied. You can return to your terminal.</span>
			</div>
		{:else}
			{#if deviceState.kind === 'invalid'}
				<div class="alert alert-error">
					<span
						>This code is invalid or has expired. Start a new login in your terminal and
						try again.</span
					>
				</div>
			{:else if deviceState.kind === 'error'}
				<div class="alert alert-error">
					<span>{deviceState.message}</span>
				</div>
			{/if}
			<form {onsubmit}>
				<p class="mb-4">Enter the code shown in your terminal.</p>
				<input
					class="input input-bordered w-full font-mono uppercase"
					type="text"
					bind:value={code}
					placeholder="XXXX-XXXX"
					autocomplete="off"
					aria-label="Device code"
					required
					disabled={busy}
				/>
				<div class="card-actions justify-end mt-4">
					<button type="submit" class="btn btn-primary" disabled={busy || !code}>
						{#if busy}
							<span class="loading loading-spinner loading-sm"></span>
						{/if}
						Continue
					</button>
				</div>
			</form>
		{/if}
	</div>
</div>
