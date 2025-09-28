<script lang="ts">
	import { connectionState, lastError, webSocketStore } from '../stores/webSocketStore';
	import { isAuthenticated } from '../stores/sessionStore';
	import Icon from '@iconify/svelte';
	import cloudCheck from '@iconify-icons/ph/cloud-check';
	import cloudWarning from '@iconify-icons/ph/cloud-warning';
	import cloudSlash from '@iconify-icons/ph/cloud-slash';
	import cloud from '@iconify-icons/ph/cloud';

	$: statusIcon = getStatusIcon($connectionState, $isAuthenticated);
	$: statusText = getStatusText($connectionState, $isAuthenticated);

	function getStatusIcon(state: string, authenticated: boolean) {
		switch (state) {
			case 'disconnected':
				return cloudSlash;
			case 'connecting':
				return cloud;
			case 'connected':
				return authenticated ? cloudCheck : cloud;
			case 'authenticated':
				return cloudCheck;
			case 'error':
				return cloudWarning;
			default:
				return cloudSlash;
		}
	}

	function getStatusText(state: string, authenticated: boolean): string {
		switch (state) {
			case 'disconnected':
				return 'Disconnected';
			case 'connecting':
				return 'Connecting...';
			case 'connected':
				return authenticated ? 'Connected' : 'Authenticating...';
			case 'authenticated':
				return 'Connected';
			case 'error':
				return 'Connection Error';
			default:
				return 'Unknown';
		}
	}

	function handleClick() {
		if ($connectionState === 'disconnected' || $connectionState === 'error') {
			webSocketStore.connect();
		}
	}
</script>

<div class="flex items-center">
	<button
		class="tooltip tooltip-top hover:opacity-75 transition-opacity"
		data-tip={$lastError || statusText}
		on:click={handleClick}
		class:cursor-pointer={$connectionState === 'disconnected' || $connectionState === 'error'}
		class:cursor-default={$connectionState !== 'disconnected' && $connectionState !== 'error'}
	>
		<Icon icon={statusIcon} class="w-4 h-4" />
	</button>
</div>