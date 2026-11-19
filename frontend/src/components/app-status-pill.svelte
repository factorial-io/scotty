<script lang="ts">
	import StatusMark, { type Tone } from './status-mark.svelte';

	/** App status (`Running`, `Stopped`, …) or container status (`running`, `exited`, …). */
	let { status = 'Running' }: { status?: string } = $props();

	const tones: Record<string, Tone> = {
		running: 'ok',
		stopped: 'off',
		exited: 'off',
		dead: 'err',
		unsupported: 'warn',
		starting: 'info',
		stopping: 'info',
		creating: 'info',
		destroying: 'info',
		restarting: 'info',
		removing: 'info'
	};

	let key = $derived(status.toLowerCase());
	let tone = $derived(tones[key] ?? 'off');
	let label = $derived(status ? status[0].toUpperCase() + status.slice(1) : 'Unknown');
</script>

<StatusMark {tone} {label} icon={tone === 'info' ? 'spinner' : undefined} />
