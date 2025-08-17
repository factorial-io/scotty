<script lang="ts">
	import { onMount } from 'svelte';
	import { getAuthMode } from '$lib';

	let userInfo: { id: string; name: string; email: string } | null = null;
	let authMode: 'dev' | 'oauth' | 'bearer' = 'bearer';

	onMount(async () => {
		authMode = await getAuthMode();
		
		if (authMode === 'oauth') {
			const userInfoJson = localStorage.getItem('user_info');
			if (userInfoJson) {
				try {
					userInfo = JSON.parse(userInfoJson);
				} catch (error) {
					console.error('Failed to parse user info:', error);
				}
			}
		}
	});

	function logout() {
		if (authMode === 'oauth') {
			localStorage.removeItem('oauth_token');
			localStorage.removeItem('user_info');
		} else if (authMode === 'bearer') {
			localStorage.removeItem('token');
		}
		window.location.href = '/login';
	}
</script>

{#if authMode === 'oauth' && userInfo}
	<div class="dropdown dropdown-end">
		<div tabindex="0" role="button" class="btn btn-ghost btn-circle avatar">
			<div class="w-8 h-8 rounded-full bg-primary text-primary-content flex items-center justify-center">
				{userInfo.name.charAt(0).toUpperCase()}
			</div>
		</div>
		<ul tabindex="0" class="menu menu-sm dropdown-content bg-base-100 rounded-box z-[1] mt-3 w-52 p-2 shadow">
			<li class="menu-title">
				<span>{userInfo.name}</span>
			</li>
			<li><span class="text-xs text-gray-500">{userInfo.email}</span></li>
			<li><hr class="my-1" /></li>
			<li><button on:click={logout}>Logout</button></li>
		</ul>
	</div>
{:else if authMode === 'bearer' || authMode === 'dev'}
	<div class="dropdown dropdown-end">
		<div tabindex="0" role="button" class="btn btn-ghost btn-sm">
			{#if authMode === 'dev'}
				Dev User
			{:else}
				User
			{/if}
		</div>
		<ul tabindex="0" class="menu menu-sm dropdown-content bg-base-100 rounded-box z-[1] mt-3 w-52 p-2 shadow">
			<li><button on:click={logout}>Logout</button></li>
		</ul>
	</div>
{/if}