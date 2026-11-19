/** Filtering, counting and sorting for the app list (spec app-list-view). */
import type { App, AppService } from '../types';

export const STATUS_FILTERS = ['All', 'Running', 'Stopped', 'Busy'] as const;
export type StatusFilter = (typeof STATUS_FILTERS)[number];
export type SortKey = 'name' | 'status';

export function matchesFilter(app: App, filter: StatusFilter): boolean {
	if (filter === 'All') return true;
	if (filter === 'Busy') return !!app.operation;
	return app.status === filter;
}

export function countByFilter(apps: App[]): Record<StatusFilter, number> {
	return Object.fromEntries(
		STATUS_FILTERS.map((f) => [f, apps.filter((a) => matchesFilter(a, f)).length])
	) as Record<StatusFilter, number>;
}

export function listApps(
	apps: App[],
	search: string,
	filter: StatusFilter,
	sortBy: SortKey
): App[] {
	const q = search.toLowerCase();
	return apps
		.filter((a) => a.name.toLowerCase().includes(q) && matchesFilter(a, filter))
		.sort((a, b) =>
			sortBy === 'name'
				? a.name.localeCompare(b.name)
				: a.status.localeCompare(b.status) || a.name.localeCompare(b.name)
		);
}

/** Start time of the most recently started service, or null. */
export function lastStartedAt(services: AppService[]): string | null {
	let latest: string | null = null;
	for (const s of services) {
		if (s.started_at && (!latest || new Date(s.started_at) > new Date(latest)))
			latest = s.started_at;
	}
	return latest;
}
