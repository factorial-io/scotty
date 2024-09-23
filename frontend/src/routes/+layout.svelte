<script>
	import 'tailwindcss/tailwind.css';
	import logo from '$lib/assets/scotty.svg';
	import { apiCall, checkIfLoggedIn } from '$lib';
	import { onMount } from 'svelte';

	let site_info = {
		domain: '...'
	};

	$: {
		onMount(async () => {
			checkIfLoggedIn();
			site_info = await apiCall('info');
		});
	}
</script>

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
						<a href="/rapidoc" target="_blank" rel="noopener noreferrer">API Documentation</a>
					</li>
				</ul>
			</div>
		</div>

		<div class="my-4">
			<slot />
		</div>
	</div>
	<footer class="px-4 pb-4">
		<p class="text-right text-sm text-gray-500">
			Brought to you by <a class="link link-secondary" href="https://factorial.io/">Factorial.io</a>
		</p>
	</footer>
</div>
