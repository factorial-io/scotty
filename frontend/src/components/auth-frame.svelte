<script lang="ts">
	import type { Snippet } from 'svelte';
	import { resolve } from '$app/paths';
	import logo from '#lib/assets/scotty.svg';
	import ScottyType from './scotty-type.svelte';
	import { siteInfo } from '../stores/siteStore';

	/** Centred card for the pages outside the shell (sign-in, device, consent). */
	let { center = false, children }: { center?: boolean; children: Snippet } = $props();
</script>

<div class="auth">
	<a class="auth-brand" href={resolve('/dashboard')}>
		<span class="logo-tile"><img src={logo} alt="" /></span>
		<span><ScottyType /><span class="inst">{$siteInfo.domain}</span></span>
	</a>
	<div class="card" class:center>
		{@render children()}
	</div>
	<div class="auth-foot">
		<span>Scotty v{$siteInfo.version}</span>
		<a class="link" href="https://factorial.io/" target="_blank" rel="noopener"
			>Brought to you by Factorial.io</a
		>
	</div>
</div>
