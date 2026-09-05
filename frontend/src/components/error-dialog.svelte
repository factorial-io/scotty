<script lang="ts">
	import { errorMessage } from '../stores/errorStore';

	let dialog: HTMLDialogElement;

	$: if (dialog && $errorMessage && !dialog.open) dialog.showModal();
</script>

<dialog bind:this={dialog} class="modal" on:close={() => errorMessage.set(null)}>
	<div class="modal-box">
		<h3 class="text-lg font-bold text-error">Action failed</h3>
		<p class="py-4">{$errorMessage}</p>
		<div class="modal-action">
			<form method="dialog">
				<button class="btn">Close</button>
			</form>
		</div>
	</div>
	<form method="dialog" class="modal-backdrop"><button>close</button></form>
</dialog>
