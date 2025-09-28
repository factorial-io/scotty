<script lang="ts">
	import '../app.css';
	import logo from '$lib/assets/scotty.svg';
	import { publicApiCall, checkIfLoggedIn } from '$lib';
	import { onMount } from 'svelte';
	import title from '../stores/titleStore';
	import UserInfo from '../components/user-info.svelte';
	import WebSocketStatus from '../components/websocket-status.svelte';
	import { authStore } from '../stores/userStore';
	import { webSocketStore } from '../stores/webSocketStore';

	type SiteInfo = {
		domain: string;
		version: string;
	};

	let site_info: SiteInfo = {
		domain: '...',
		version: '0.0.0'
	};

	onMount(async () => {
		// Initialize the auth store first
		await authStore.init();

		// Initialize WebSocket store (will connect when user logs in)
		webSocketStore.initialize();

		checkIfLoggedIn();
		site_info = (await publicApiCall('info')) as SiteInfo;
	});
</script>

<svelte:head>
	<title>{$title}</title>
</svelte:head>

<div class="my-4 max-w-screen-lg mx-auto">
	<div class="my-4 p-4 border-2 border-gray-100 dark:border-gray-900 rounded shadow-sm">
		<div class="navbar bg-base-100 border-b-2 border-gray-100 dark:border-gray-900">
			<div class="flex-1">
				<div class="avatar">
					<div class="w-16 rounded-full bg-gray-50 dark:bg-gray-950">
						<img alt="Scotty Logo" src={logo} />
					</div>
				</div>
				<a href="/dashboard" class="text-xl ml-4 font-bold">scotty @ {site_info.domain}</a>
			</div>
			<div class="flex-none">
				<ul class="menu menu-horizontal px-1">
					<li>
						<a href="/dashboard">Apps</a>
					</li>
					<li>
						<a href="/tasks">Tasks</a>
					</li>
					<li>
						<a href="/rapidoc" target="_blank" rel="noopener noreferrer"
							>API Documentation</a
						>
					</li>
				</ul>
				<UserInfo />
			</div>
		</div>

		<div class="my-4">
			<slot />
		</div>
	</div>
	<footer class="px-4 pb-4 flex justify-between items-center">
		<p class="text-sm text-gray-500">
			Scotty <a
				class="link link-secondary"
				href={`https://github.com/factorial-io/scotty/releases/tag/v${site_info.version}`}
				>v{site_info.version}</a
			>
		</p>
		<div class="flex items-center space-x-4">
			<WebSocketStatus />
			<p class="text-sm text-gray-500">
				Brought to you by <a class="link link-secondary" href="https://factorial.io/"
					>Factorial.io</a
				>
			</p>
		</div>
	</footer>
</div>
