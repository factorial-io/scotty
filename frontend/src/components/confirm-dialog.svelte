<script lang="ts">
	import Icon from '@iconify/svelte';
	import { pendingConfirm, answerConfirm } from '../stores/confirmStore';

	let dialog: HTMLDialogElement | undefined = $state();
	let safeButton: HTMLButtonElement | undefined = $state();
	let confirmed = false;

	$effect(() => {
		if (!dialog) return;
		if ($pendingConfirm && !dialog.open) {
			confirmed = false;
			dialog.showModal();
			safeButton?.focus(); // the safe choice first
		} else if (!$pendingConfirm && dialog.open) {
			dialog.close();
		}
	});

	function onclose() {
		// Escape, the close icon and "Keep it" all land here without confirming.
		if ($pendingConfirm) answerConfirm(confirmed);
	}

	function confirm() {
		confirmed = true;
		dialog?.close();
	}
</script>

<dialog bind:this={dialog} class="dialog" aria-labelledby="confirm-dialog-title" {onclose}>
	{#if $pendingConfirm}
		<div class="dh">
			<h2 id="confirm-dialog-title">{$pendingConfirm.title}</h2>
			<form method="dialog">
				<button class="ibtn" aria-label={$pendingConfirm.cancelLabel ?? 'Keep it'}>
					<Icon icon="ph:x" width="24" height="24" />
				</button>
			</form>
		</div>
		<div class="db">
			<p style="margin:0">{$pendingConfirm.body}</p>
			{#if $pendingConfirm.consequence}
				<div class="consequence">{$pendingConfirm.consequence}</div>
			{/if}
		</div>
		<div class="df">
			<form method="dialog">
				<button class="btn outline" bind:this={safeButton}
					>{$pendingConfirm.cancelLabel ?? 'Keep it'}</button
				>
			</form>
			<button class="btn danger" onclick={confirm}>
				<Icon icon="ph:trash" width="24" height="24" />{$pendingConfirm.confirmLabel}
			</button>
		</div>
	{/if}
</dialog>
