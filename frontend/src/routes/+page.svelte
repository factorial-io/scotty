<script>
	import { onMount } from 'svelte';

	onMount(() => {
		const token = localStorage.getItem('token');

		if (!token) {
			// If token is not found, redirect to the login page
			window.location.href = '/login';
		} else {
			// Optionally, you can add further validation of the token by making an API call
			validateToken(token);
		}
	});

	/**
	 * @param {string} token
	 */
	async function validateToken(token) {
		const response = await fetch('/api/v1/validate-token', {
			method: 'POST',
			headers: {
				Authorization: `Bearer ${token}`
			}
		});

		if (!response.ok) {
			// If the token is invalid, redirect to login page
			window.location.href = '/login';
		}
		window.location.href = '/dashboard';
	}
</script>
