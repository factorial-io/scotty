import { describe, expect, it } from 'vitest';
import { formatRelativeTime } from '$lib/relative-time';

const now = new Date('2026-09-05T12:00:00Z');
const ago = (seconds: number) => new Date(now.getTime() - seconds * 1000).toISOString();

describe('formatRelativeTime', () => {
	it('picks the unit by magnitude', () => {
		expect(formatRelativeTime(now, ago(0))).toBe('0 seconds ago');
		expect(formatRelativeTime(now, ago(59))).toBe('59 seconds ago');
		expect(formatRelativeTime(now, ago(60))).toBe('1 minutes ago');
		expect(formatRelativeTime(now, ago(3599))).toBe('59 minutes ago');
		expect(formatRelativeTime(now, ago(3600))).toBe('1 hours ago');
		expect(formatRelativeTime(now, ago(86399))).toBe('23 hours ago');
		expect(formatRelativeTime(now, ago(86400))).toBe('1 days ago');
		expect(formatRelativeTime(now, ago(30 * 86400))).toBe('30 days ago');
	});

	it('formats future timestamps with the same units', () => {
		expect(formatRelativeTime(now, ago(-90))).toBe('in 1 minutes');
		expect(formatRelativeTime(now, ago(-2 * 86400))).toBe('in 2 days');
	});
});
