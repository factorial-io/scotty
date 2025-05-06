<script>
	import { onMount } from 'svelte';
	import { setTitle } from '../stores/titleStore';

	onMount(() => {
		setTitle('Redirecting to Dashboard');
		window.location.href = '/dashboard';
	});
</script>
