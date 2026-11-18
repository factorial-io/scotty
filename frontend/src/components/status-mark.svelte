<script lang="ts">
	import Icon from '@iconify/svelte';

	export type Tone = 'ok' | 'err' | 'warn' | 'info' | 'off';

	interface Props {
		tone: Tone;
		label: string;
		/** Phosphor icon instead of the dot, e.g. `ph:check-circle`; `spinner` spins. */
		icon?: string;
		title?: string;
	}

	let { tone, label, icon, title }: Props = $props();
</script>

<!-- Status is always colour plus word (app design rule). -->
<span class="st {tone}" {title}>
	{#if icon === 'spinner'}
		<Icon icon="ph:spinner" class="ic sm spin" aria-hidden="true" />
	{:else if icon}
		<Icon {icon} class="ic sm" aria-hidden="true" />
	{:else}
		<i class="sw"></i>
	{/if}
	<span class="w">{label}</span>
</span>
