<script lang="ts">
	import { connectionState, lastError, webSocketStore } from '../stores/webSocketStore';
	import { isAuthenticated } from '../stores/sessionStore';

	const states: Record<string, { tone: string; text: string }> = {
		disconnected: { tone: 'err', text: 'Disconnected, click to reconnect' },
		connecting: { tone: 'info', text: 'Connecting' },
		authenticated: { tone: 'ok', text: 'Connected' },
		error: { tone: 'err', text: 'Connection error, click to reconnect' }
	};

	let s = $derived(
		$connectionState === 'connected'
			? $isAuthenticated
				? states.authenticated
				: { tone: 'info', text: 'Authenticating' }
			: (states[$connectionState] ?? states.disconnected)
	);
	let broken = $derived($connectionState === 'disconnected' || $connectionState === 'error');
</script>

<button
	class="live st {s.tone}"
	title={$lastError || s.text}
	aria-label="Live updates: {$lastError || s.text}"
	onclick={() => broken && webSocketStore.connect()}
	style:cursor={broken ? 'pointer' : 'default'}
>
	<i class="sw"></i><span>Live updates</span>
</button>
