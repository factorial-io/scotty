<script lang="ts">
	import Icon from '@iconify/svelte';
	import { errorMessage } from '../stores/errorStore';

	let dialog: HTMLDialogElement | undefined = $state();

	$effect(() => {
		if (dialog && $errorMessage && !dialog.open) dialog.showModal();
	});
</script>

<!-- Global error dialog for failed actions (app-action-feedback). -->
<dialog
	bind:this={dialog}
	class="dialog"
	aria-labelledby="error-dialog-title"
	onclose={() => errorMessage.set(null)}
>
	<div class="dh">
		<h2 id="error-dialog-title" style="color: var(--color-status-error)">Action failed</h2>
		<form method="dialog">
			<button class="ibtn" aria-label="Close"
				><Icon icon="ph:x" width="24" height="24" /></button
			>
		</form>
	</div>
	<div class="db"><p style="margin:0">{$errorMessage}</p></div>
	<div class="df">
		<form method="dialog"><button class="btn outline">Close</button></form>
	</div>
</dialog>
