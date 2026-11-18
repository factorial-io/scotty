import { describe, expect, it } from 'vitest';
import { countByFilter, lastStartedAt, listApps } from './appList';
import { formatRelativeTime } from './relative-time';
import type { App, AppService } from '../types';

const app = (name: string, status: string, operation?: object) =>
	({ name, status, services: [], operation }) as unknown as App;
const apps = [
	app('shop', 'Running'),
	app('shop-api', 'Stopped'),
	app('blog', 'Running'),
	app('wiki', 'Running', { operation: 'rebuild' }),
	app('legacy', 'Unsupported')
];

describe('app list', () => {
	it('counts each filter, Busy by operation', () => {
		expect(countByFilter(apps)).toEqual({ All: 5, Running: 3, Stopped: 1, Busy: 1 });
	});

	it('follows a live operation change', () => {
		const live = [...apps.slice(0, 4), app('legacy', 'Unsupported', { operation: 'run' })];
		expect(countByFilter(live).Busy).toBe(2);
		expect(listApps(live, '', 'Busy', 'name').map((a) => a.name)).toEqual(['legacy', 'wiki']);
	});

	it('combines a case-insensitive search with the filter', () => {
		expect(listApps(apps, 'SHOP', 'Running', 'name').map((a) => a.name)).toEqual(['shop']);
	});

	it('sorts by status, then name', () => {
		expect(listApps(apps, '', 'All', 'status').map((a) => a.name)).toEqual([
			'blog',
			'shop',
			'wiki',
			'shop-api',
			'legacy'
		]);
	});

	it('takes the most recently started service', () => {
		const now = new Date('2026-11-19T12:00:00Z');
		const at = (h: number) => new Date(now.getTime() - h * 3600e3).toISOString();
		const services = [{ started_at: at(40 * 24) }, { started_at: at(3) }] as AppService[];
		expect(formatRelativeTime(now, lastStartedAt(services)!)).toBe('3 h ago');
	});
});
