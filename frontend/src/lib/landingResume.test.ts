import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('$app/navigation', () => ({ goto: vi.fn() }));
vi.mock('$app/paths', () => ({ resolve: (p: string) => p }));

import { goto } from '$app/navigation';
import {
	consumePendingLandingRedirect,
	redirectAfterLogin,
	storePendingLandingStart
} from '$lib/landingResume';

const KEY = 'scotty_landing_pending_start';

describe('landing resume state', () => {
	beforeEach(() => {
		sessionStorage.clear();
		vi.mocked(goto).mockClear();
	});

	it('returns null when nothing is pending', () => {
		expect(consumePendingLandingRedirect()).toBeNull();
	});

	it('round-trips a pending start and clears it', () => {
		storePendingLandingStart({ appName: 'my app', returnUrl: 'https://x.test/a?b=1' });
		expect(consumePendingLandingRedirect()).toBe(
			'/landing/my%20app?autostart=true&return_url=https%3A%2F%2Fx.test%2Fa%3Fb%3D1'
		);
		expect(sessionStorage.getItem(KEY)).toBeNull();
		expect(consumePendingLandingRedirect()).toBeNull();
	});

	it('omits return_url when none was stored', () => {
		storePendingLandingStart({ appName: 'demo', returnUrl: null });
		expect(consumePendingLandingRedirect()).toBe('/landing/demo?autostart=true');
	});

	it('ignores malformed or incomplete entries and still clears them', () => {
		sessionStorage.setItem(KEY, '{not json');
		expect(consumePendingLandingRedirect()).toBeNull();
		expect(sessionStorage.getItem(KEY)).toBeNull();

		sessionStorage.setItem(KEY, JSON.stringify({ returnUrl: '/x' }));
		expect(consumePendingLandingRedirect()).toBeNull();
	});

	it('redirects to the landing page when pending, else to the dashboard', async () => {
		await redirectAfterLogin();
		expect(goto).toHaveBeenLastCalledWith('/dashboard');

		storePendingLandingStart({ appName: 'demo', returnUrl: null });
		await redirectAfterLogin();
		expect(goto).toHaveBeenLastCalledWith('/landing/demo?autostart=true');
	});
});
