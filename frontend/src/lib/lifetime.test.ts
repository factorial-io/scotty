import { describe, expect, it } from 'vitest';
import { formatLifetime } from './lifetime';
import type { App } from '../types';

const now = new Date('2026-01-10T00:00:00Z');
const app = (time_to_live: unknown, destroy_on_ttl: boolean, ttl_expires_at: string | null) =>
	({ status: 'Running', settings: { time_to_live, destroy_on_ttl }, ttl_expires_at }) as App;

describe('formatLifetime', () => {
	it('counts down a running app', () => {
		expect(formatLifetime(app({ Days: 7 }, true, '2026-01-13T00:00:00Z'), now)).toBe(
			'Lifetime: 7 days, then destroyed in 3 d'
		);
	});
	it('says when the app is not running or overdue', () => {
		expect(formatLifetime(app({ Hours: 12 }, false, null), now)).toBe(
			'Lifetime: 12 hours, then stopped (not running)'
		);
		expect(formatLifetime(app({ Hours: 12 }, false, '2026-01-09T00:00:00Z'), now)).toBe(
			'Lifetime: 12 hours, then stopped (due now)'
		);
	});
	it('never destroys an unsupported app', () => {
		const a = { ...app({ Days: 1 }, true, null), status: 'Unsupported' } as App;
		expect(formatLifetime(a, now)).toBe('Lifetime: 1 days, then stopped (not running)');
	});
	it('shows forever', () => {
		expect(formatLifetime(app('Forever', true, null), now)).toBe('Lifetime: Forever');
	});
});
