<script lang="ts">
	import type { Snippet } from 'svelte';
	import Icon from '@iconify/svelte';
	import { Tooltip } from 'bits-ui';

	interface Props {
		href: string;
		icon: string;
		label: string;
		active?: boolean;
		/** Icons-only rail: the label shows as a tooltip. */
		rail?: boolean;
		external?: boolean;
		count?: Snippet;
	}

	let {
		href,
		icon,
		label,
		active = false,
		rail = false,
		external = false,
		count
	}: Props = $props();
</script>

<Tooltip.Root disabled={!rail} delayDuration={200}>
	<Tooltip.Trigger>
		{#snippet child({ props })}
			<!-- eslint-disable svelte/no-navigation-without-resolve -- callers pass resolved paths -->
			<a
				{...props}
				class="item"
				class:active
				aria-current={active ? 'page' : undefined}
				aria-label={rail ? label : undefined}
				{href}
				target={external ? '_blank' : undefined}
				rel={external ? 'noopener noreferrer' : undefined}
			>
				<Icon {icon} class="ic" aria-hidden="true" />
				<span class="label">{label}</span>
				{@render count?.()}
			</a>
		{/snippet}
	</Tooltip.Trigger>
	<Tooltip.Portal>
		<Tooltip.Content class="tip" side="right" sideOffset={8}>{label}</Tooltip.Content>
	</Tooltip.Portal>
</Tooltip.Root>
