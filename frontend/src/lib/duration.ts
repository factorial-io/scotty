/** Task duration: "62 s", "14 min", "3 h". */
export function formatDuration(start: string, end: string): string {
	const s = Math.max(1, Math.round((new Date(end).getTime() - new Date(start).getTime()) / 1000));
	if (s < 120) return `${s} s`;
	if (s < 7200) return `${Math.round(s / 60)} min`;
	return `${Math.round(s / 3600)} h`;
}
