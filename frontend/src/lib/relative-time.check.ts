/**
 * Self-check for `formatRelativeTime`. Run with `bun src/lib/relative-time.check.ts`.
 *
 * Deliberately framework-free: the frontend has no test runner, and one pure function
 * does not justify adding one. Plain throws keep this type-checkable by svelte-check,
 * which cannot resolve `bun:test` without pulling in @types/bun.
 */
import { formatRelativeTime } from './relative-time';

const now = new Date('2026-07-29T12:00:00Z');

function at(offsetSeconds: number) {
	return new Date(now.getTime() + offsetSeconds * 1000).toISOString();
}

function check(offsetSeconds: number, expected: string) {
	const actual = formatRelativeTime(now, at(offsetSeconds));
	if (actual !== expected) {
		throw new Error(`offset ${offsetSeconds}s: expected "${expected}", got "${actual}"`);
	}
}

// Future timestamps read forward.
check(30, 'in 30 seconds');
check(12 * 60, 'in 12 minutes');
check(3 * 3600, 'in 3 hours');
check(2 * 86400, 'in 2 days');

// Past timestamps read backward, unchanged from the original component behaviour.
check(-30, '30 seconds ago');
check(-12 * 60, '12 minutes ago');
check(-3 * 3600, '3 hours ago');
check(-2 * 86400, '2 days ago');

// The present is not forward-looking.
check(0, '0 seconds ago');

console.log('relative-time: all checks passed');
