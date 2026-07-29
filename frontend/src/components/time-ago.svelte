<script lang="ts">
	import { time } from '../stores/time';
	import { formatRelativeTime } from '../lib/relative-time';
	export let dateString: string | null = null;

	let timeAgoText: string = '';

	$: {
		timeAgoText = dateString ? formatRelativeTime($time, dateString) : '';
	}
</script>

<span>{timeAgoText}</span>
