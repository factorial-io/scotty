<script>
	import logo from '$lib/assets/scotty.svg';
	import { onMount } from 'svelte';
	import { setTitle } from '../../stores/titleStore';

	let password = '';

	onMount(() => {
		setTitle('Login');
	});

	async function login() {
		const response = await fetch('/api/v1/login', {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json'
			},
			body: JSON.stringify({ password })
		});

		if (response.ok) {
			const { token } = await response.json();
			localStorage.setItem('token', token); // Store token in localStorage
			window.location.href = '/dashboard'; // Redirect to dashboard
		} else {
			alert('Login failed');
		}
	}
</script>

<div class="card bg-base-100 w-96 shadow-md mx-auto">
	<figure class="bg-gray-100 max-h-48">
		<img src={logo} alt="Scotty Logo" />
	</figure>
	<div class="card-body">
		<form on:submit|preventDefault={login}>
			<div class="card-body">
				<h2 class="card-title">Please log in</h2>
				<p>Please provide the password for this installation</p>
				<div class="my-4">
					<input
						class="input input-bordered w-full max-w-xs"
						type="password"
						bind:value={password}
						placeholder="Enter your password"
					/>
				</div>
			</div>
			<div class="card-actions justify-end">
				<button type="submit" class="btn btn-primary">Login</button>
			</div>
		</form>
	</div>
</div>
