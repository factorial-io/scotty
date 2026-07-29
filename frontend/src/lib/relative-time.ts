/**
 * Formats a timestamp relative to `now`, in either direction: past timestamps read
 * "12 minutes ago", future ones "in 12 minutes". Unit selection is identical for
 * both directions, so a countdown and a staleness reading look alike.
 */
export function formatRelativeTime(now: Date, dateString: string): string {
	const diffSeconds = Math.floor((now.getTime() - new Date(dateString).getTime()) / 1000);
	const magnitude = Math.abs(diffSeconds);

	let value: number;
	let unit: string;
	if (magnitude < 60) {
		value = magnitude;
		unit = 'seconds';
	} else if (magnitude < 3600) {
		value = Math.floor(magnitude / 60);
		unit = 'minutes';
	} else if (magnitude < 86400) {
		value = Math.floor(magnitude / 3600);
		unit = 'hours';
	} else {
		value = Math.floor(magnitude / 86400);
		unit = 'days';
	}

	return diffSeconds < 0 ? `in ${value} ${unit}` : `${value} ${unit} ago`;
}
