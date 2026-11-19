<script lang="ts">
	import Icon from '@iconify/svelte';
	import { toasts, dismissToast, pauseToast, resumeToast } from '../stores/toastStore';
</script>

<!-- Top centre, one line, dark in every theme (rebrush.css). -->
<div class="toasts" aria-live="polite">
	{#each $toasts as t (t.id)}
		<div
			class="toast"
			role="status"
			onmouseenter={() => pauseToast(t.id)}
			onmouseleave={() => resumeToast(t.id)}
		>
			<span class="t">{t.message}</span>
			<button
				class="ibtn"
				aria-label="Close"
				style="width:24px;height:24px"
				onclick={() => dismissToast(t.id)}
			>
				<Icon icon="ph:x" width="16" height="16" />
			</button>
		</div>
	{/each}
</div>
