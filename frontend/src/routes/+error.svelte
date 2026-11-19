<script lang="ts">
	import Icon from '@iconify/svelte';
	import { page } from '$app/state';
	import { resolve } from '$app/paths';

	let notFound = $derived(page.status === 404);
	// The server answers 403 for apps you may not see, existing or not.
	let forbidden = $derived(page.status === 403);
</script>

<!-- Rendered inside the shell, so navigation and breadcrumbs stay. -->
<div class="empty">
	<div class="sq">
		<Icon
			icon={notFound || forbidden ? 'ph:magnifying-glass' : 'ph:warning'}
			class="ic lg"
			aria-hidden="true"
		/>
	</div>
	<h2>{notFound ? 'Not found' : forbidden ? 'Not available' : 'Something went wrong'}</h2>
	<p>
		{notFound
			? 'It may have been removed, or the server restarted and forgot it.'
			: forbidden
				? 'It does not exist, or you have no access to it.'
				: page.error?.message}
	</p>
	<p><a class="link" href={resolve('/dashboard')}>Back to apps</a></p>
</div>
