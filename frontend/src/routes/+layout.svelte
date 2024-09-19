<script>
	import 'tailwindcss/tailwind.css';
	import logo from '$lib/assets/scotty.svg';
</script>

<div class="m-4 p-4 border-2 border-gray-100 rounded">
	<div class="navbar bg-base-100 border-b-2 border-gray-100">
		<div class="flex-1">
			<div class="avatar">
				<div class="w-16 rounded-full bg-gray-50">
					<img alt="Scotty Logo" src={logo} />
				</div>
			</div>
			<a href="/" class="btn btn-ghost text-xl">scotty</a>
		</div>
		<div class="flex-none">
			<ul class="menu menu-horizontal px-1">
				<li><a href="/rapidocs">API Documentation</a></li>
			</ul>
		</div>
	</div>

	<div class="m-4 p-4">
		<slot />
	</div>
</div>
