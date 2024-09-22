<script>
	import { onMount, onDestroy } from 'svelte';

	export let dateString;

	let timeAgoText = '';
	let interval;

	function getTimeAgo(dateString) {
		const now = new Date();
		const past = new Date(dateString);
		const diffSeconds = Math.floor((now - past) / 1000);

		if (dateString === null || isNaN(diffSeconds)) {
			return '';
		} else if (diffSeconds < 60) {
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
		timeAgoText = getTimeAgo(dateString);
		scheduleNextUpdate();
	}

	function scheduleNextUpdate() {
		const now = new Date();
		const past = new Date(dateString);
		const diffSeconds = Math.floor((now - past) / 1000);

		if (interval) clearInterval(interval);

		if (past == null || isNaN(diffSeconds) || diffSeconds < 60) {
			interval = setInterval(updateTime, 1000);
		} else {
			interval = setInterval(updateTime, 60000);
		}
	}

	onMount(() => {
		updateTime();
	});

	onDestroy(() => {
		if (interval) clearInterval(interval);
	});
</script>

<span>{timeAgoText}</span>
