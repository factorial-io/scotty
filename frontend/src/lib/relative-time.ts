/**
 * Formats a timestamp relative to `now`, in either direction: past timestamps read
 * "12 min ago", future ones "in 12 min". Unit selection is identical for
 * both directions, so a countdown and a staleness reading look alike.
 */
export function formatRelativeTime(now: Date, dateString: string): string {
	const diffSeconds = Math.floor((now.getTime() - new Date(dateString).getTime()) / 1000);
	const magnitude = Math.abs(diffSeconds);

	let value: number;
	let unit: string;
	if (magnitude < 60) {
		value = magnitude;
		unit = 's';
	} else if (magnitude < 3600) {
		value = Math.floor(magnitude / 60);
		unit = 'min';
	} else if (magnitude < 86400) {
		value = Math.floor(magnitude / 3600);
		unit = 'h';
	} else {
		value = Math.floor(magnitude / 86400);
		unit = 'd';
	}

	return diffSeconds < 0 ? `in ${value} ${unit}` : `${value} ${unit} ago`;
}
