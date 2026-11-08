import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('$app/navigation', () => ({ goto: vi.fn() }));
vi.mock('$app/paths', () => ({ resolve: (p: string) => p }));
vi.mock('../stores/sessionStore', () => ({
	sessionStore: {
		getAuthHeader: vi.fn(() => ({ Authorization: 'Bearer session-token' })),
		logout: vi.fn()
	}
}));

import { goto } from '$app/navigation';
import { sessionStore } from '../stores/sessionStore';
import { authService } from './authService';
import { consumeReturnPath } from './landingResume';

describe('authService', () => {
	const fetchMock = vi.fn();

	beforeEach(() => {
		sessionStorage.clear();
		vi.stubGlobal('fetch', fetchMock);
		fetchMock.mockReset();
		vi.mocked(goto).mockClear();
		vi.mocked(sessionStore.logout).mockClear();
		vi.spyOn(console, 'warn').mockImplementation(() => {});
	});

	afterEach(() => {
		vi.unstubAllGlobals();
	});

	it('revokes the session at the server before clearing it locally', async () => {
		fetchMock.mockResolvedValue(new Response(null, { status: 204 }));

		await authService.logout('/login');

		const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit];
		expect(url).toBe('/api/v1/authenticated/logout');
		expect(init.method).toBe('POST');
		expect(init.headers).toMatchObject({ Authorization: 'Bearer session-token' });
		expect(sessionStore.logout).toHaveBeenCalledOnce();
		expect(goto).toHaveBeenCalledWith('/login');
	});

	it('still logs out locally when the revoke call fails', async () => {
		fetchMock.mockRejectedValue(new Error('offline'));

		await authService.logout('/login');

		expect(sessionStore.logout).toHaveBeenCalledOnce();
		expect(goto).toHaveBeenCalledWith('/login');
	});

	it('remembers the current page when sending the user to login', async () => {
		window.history.replaceState(null, '', '/device?code=ABCD-EFGH');

		await authService.redirectToLogin();

		expect(goto).toHaveBeenCalledWith('/login');
		expect(consumeReturnPath()).toBe('/device?code=ABCD-EFGH');
	});
});
