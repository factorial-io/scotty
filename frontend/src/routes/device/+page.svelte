<script lang="ts">
	import { onMount } from 'svelte';
	import Icon from '@iconify/svelte';
	import { resolve } from '$app/paths';
	import AuthFrame from '../../components/auth-frame.svelte';
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

<AuthFrame center={deviceState.kind === 'approved' || deviceState.kind === 'denied'}>
	{#if deviceState.kind === 'pending'}
		<h1>Authorize a device</h1>
		<p>
			<b>{deviceState.request.client_name}</b> wants to access Scotty as
			<b>{$userInfo?.email || 'you'}</b>.
		</p>
		<dl class="dl">
			<div>
				<dt>Code</dt>
				<dd class="mono">{deviceState.request.user_code}</dd>
			</div>
			{#if scopeList(deviceState.request.scope).length > 0}
				<div>
					<dt>Requested access</dt>
					<dd class="mono">
						{#each scopeList(deviceState.request.scope) as scope (scope)}{scope}<br
							/>{/each}
					</dd>
				</div>
			{/if}
		</dl>
		<div class="banner warn">
			<Icon icon="ph:warning" class="ic" aria-hidden="true" />
			<div class="txt">
				<b>Only approve if you started this login yourself</b>
				<span>Someone could have sent you this code to get access as you.</span>
			</div>
		</div>
		<div class="acts">
			<button class="btn outline" onclick={() => decide(false)}>Deny</button>
			<button class="btn filled primary" onclick={() => decide(true)}>Approve</button>
		</div>
	{:else if deviceState.kind === 'approved'}
		<div class="ok-mark" style="color:var(--color-status-success)">
			<Icon icon="ph:check-circle" class="ic lg" aria-hidden="true" />
		</div>
		<h1>Device approved</h1>
		<p class="sub2">
			You can go back to your terminal. The connection now shows under Connections.
		</p>
		<div class="acts">
			<a class="btn outline" href={resolve('/connections')}>View connections</a>
		</div>
	{:else if deviceState.kind === 'denied'}
		<div class="ok-mark" style="color:var(--color-status-warning)">
			<Icon icon="ph:x" class="ic lg" aria-hidden="true" />
		</div>
		<h1>Access denied</h1>
		<p class="sub2">The device did not get access. You can go back to your terminal.</p>
	{:else}
		<h1>Authorize a device</h1>
		{#if deviceState.kind === 'invalid'}
			<div class="banner err">
				<Icon icon="ph:warning-circle" class="ic" aria-hidden="true" />
				<div class="txt">
					<b>This code is invalid or has expired</b>
					<span>Start a new login in your terminal and enter the new code.</span>
				</div>
			</div>
		{:else if deviceState.kind === 'error'}
			<div class="banner err">
				<Icon icon="ph:warning-circle" class="ic" aria-hidden="true" />
				<div class="txt"><b>Something went wrong</b><span>{deviceState.message}</span></div>
			</div>
		{/if}
		<p class="sub2">
			Enter the code shown in your terminal after running <span class="mono"
				>scottyctl auth:login</span
			>.
		</p>
		<form {onsubmit} style="display:flex;flex-direction:column;gap:20px">
			<input
				class="code-in"
				type="text"
				bind:value={code}
				placeholder="XXXX-XXXX"
				autocomplete="off"
				aria-label="Device code"
				required
				disabled={busy}
			/>
			<div class="acts">
				<button type="submit" class="btn filled primary" disabled={busy || !code}>
					{#if busy}<Icon icon="ph:spinner" class="ic sm spin" aria-hidden="true" />{/if}
					Continue
				</button>
			</div>
		</form>
	{/if}
</AuthFrame>
