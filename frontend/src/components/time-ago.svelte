<script lang="ts">
	import { time } from '../stores/time';
	export let dateString: string | null = null;

	let timeAgoText: string = '';

	function getTimeAgo(now: Date, dateString: string) {
		const past = new Date(dateString);
		const diffSeconds = Math.floor((now.getTime() - past.getTime()) / 1000);

		if (diffSeconds < 60) {
			return `${diffSeconds} seconds ago`;
		} else if (diffSeconds < 3600) {
			const diffMinutes = Math.floor(diffSeconds / 60);
			return `${diffMinutes} minutes ago`;
		} else if (diffSeconds < 86400) {
			const diffHours = Math.floor(diffSeconds / 3600);
			return `${diffHours} hours ago`;
		} else {
			const diffDays = Math.floor(diffSeconds / 86400);
			return `${diffDays} days ago`;
		}
	}

	$: {
		timeAgoText = dateString ? getTimeAgo($time, dateString) : '';
	}
</script>

<span>{timeAgoText}</span>
