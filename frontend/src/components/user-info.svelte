<script lang="ts">
	import { authStore, userInfo, authMode } from '../stores/userStore';
	import UserAvatar from './user-avatar.svelte';

	function logout() {
		const currentAuthMode = $authMode;

		// Clear localStorage
		if (currentAuthMode === 'oauth') {
			localStorage.removeItem('oauth_token');
			localStorage.removeItem('user_info');
		} else if (currentAuthMode === 'bearer') {
			localStorage.removeItem('token');
		}

		// Update the store
		authStore.logout();

		// Redirect to login
		window.location.href = '/login';
	}
</script>

{#if $authMode === 'oauth' && $userInfo}
	<div class="dropdown dropdown-end">
		<div tabindex="0" role="button" class="btn btn-ghost btn-circle">
			<UserAvatar
				email={$userInfo.email || ''}
				name={$userInfo.name || $userInfo.id || 'User'}
				size="sm"
			/>
		</div>
		<ul
			tabindex="0"
			class="menu menu-sm dropdown-content bg-base-100 rounded-box z-[1] mt-3 w-64 p-2 shadow"
		>
			<li class="px-2 py-3">
				<div class="flex items-center gap-3">
					<UserAvatar
						email={$userInfo.email || ''}
						name={$userInfo.name || $userInfo.id || 'User'}
						size="md"
					/>
					<div class="flex flex-col">
						<span class="font-medium text-sm"
							>{$userInfo.name || $userInfo.id || 'Unknown User'}</span
						>
						<span class="text-xs text-gray-500"
							>{$userInfo.email || 'No email provided'}</span
						>
					</div>
				</div>
			</li>
			<li><hr class="my-1" /></li>
			<li>
				<button on:click={logout} class="text-left">
					<svg
						xmlns="http://www.w3.org/2000/svg"
						fill="none"
						viewBox="0 0 24 24"
						stroke-width="1.5"
						stroke="currentColor"
						class="w-4 h-4"
					>
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							d="M15.75 9V5.25A2.25 2.25 0 0013.5 3h-6a2.25 2.25 0 00-2.25 2.25v13.5A2.25 2.25 0 007.5 21h6a2.25 2.25 0 002.25-2.25V15M12 9l-3 3m0 0l3 3m-3-3h12.75"
						/>
					</svg>
					Logout
				</button>
			</li>
		</ul>
	</div>
{:else if $authMode === 'bearer' || $authMode === 'dev'}
	<div class="dropdown dropdown-end">
		<div tabindex="0" role="button" class="btn btn-ghost btn-sm gap-2">
			<UserAvatar email="" name={$authMode === 'dev' ? 'Dev User' : 'User'} size="xs" />
			{#if $authMode === 'dev'}
				Dev User
			{:else}
				User
			{/if}
		</div>
		<ul
			tabindex="0"
			class="menu menu-sm dropdown-content bg-base-100 rounded-box z-[1] mt-3 w-52 p-2 shadow"
		>
			<li>
				<button on:click={logout} class="text-left">
					<svg
						xmlns="http://www.w3.org/2000/svg"
						fill="none"
						viewBox="0 0 24 24"
						stroke-width="1.5"
						stroke="currentColor"
						class="w-4 h-4"
					>
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							d="M15.75 9V5.25A2.25 2.25 0 0013.5 3h-6a2.25 2.25 0 00-2.25 2.25v13.5A2.25 2.25 0 007.5 21h6a2.25 2.25 0 002.25-2.25V15M12 9l-3 3m0 0l3 3m-3-3h12.75"
						/>
					</svg>
					Logout
				</button>
			</li>
		</ul>
	</div>
{/if}
