<script lang="ts">
	import { onMount, onDestroy } from 'svelte';

	export let dateString: string | null = null;

	let timeAgoText: string = '';
	let interval: number;

	function getTimeAgo(dateString: string) {
		const now = new Date();
		const past = new Date(dateString);
		const diffSeconds = Math.floor((now.getTime() - past.getTime()) / 1000);

		if (diffSeconds < 60) {
			return `${diffSeconds} seconds ago`;
		} else if (diffSeconds < 3600) {
			const diffMinutes = Math.floor(diffSeconds / 60);
			return `${diffMinutes} minutes ago`;
		} else {
			const diffHours = Math.floor(diffSeconds / 3600);
			return `${diffHours} hours ago`;
		}
	}

	function updateTime() {
		timeAgoText = dateString ? getTimeAgo(dateString) : '';
		scheduleNextUpdate(dateString);
	}

	function scheduleNextUpdate(dateString: string | null) {
		let milliseconds: number;
		if (dateString == null) {
			milliseconds = 1000;
		} else {
			const now = new Date();
			const past = new Date(dateString);
			const diffSeconds = Math.floor((now.getTime() - past.getTime()) / 1000);
			if (past == null || isNaN(diffSeconds) || diffSeconds < 60) {
				milliseconds = 1000;
			} else {
				milliseconds = 60000;
			}
		}

		if (interval) clearInterval(interval);
		interval = setInterval(updateTime, milliseconds);
	}

	onMount(() => {
		updateTime();
	});

	onDestroy(() => {
		if (interval) clearInterval(interval);
	});
</script>

<span>{timeAgoText}</span>
