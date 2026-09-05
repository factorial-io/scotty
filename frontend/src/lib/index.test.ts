import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../stores/sessionStore', () => ({
	sessionStore: {
		isAuthenticated: vi.fn(() => true),
		init: vi.fn(async () => {}),
		getAuthHeader: vi.fn(() => ({ Authorization: 'Bearer test-token' })),
		clearInvalidSession: vi.fn(async () => {})
	}
}));
vi.mock('./authService', () => ({ authService: { logout: vi.fn(async () => {}) } }));

import { sessionStore } from '../stores/sessionStore';
import { authService } from './authService';
import { authenticatedApiCall } from '$lib';

function jsonResponse(status: number, body: unknown, statusText = '') {
	return new Response(JSON.stringify(body), {
		status,
		statusText,
		headers: { 'Content-Type': 'application/json' }
	});
}

describe('authenticatedApiCall', () => {
	const fetchMock = vi.fn();

	beforeEach(() => {
		vi.stubGlobal('fetch', fetchMock);
		fetchMock.mockReset();
		vi.spyOn(console, 'error').mockImplementation(() => {});
		vi.spyOn(console, 'warn').mockImplementation(() => {});
	});

	afterEach(() => {
		vi.unstubAllGlobals();
		vi.restoreAllMocks();
	});

	it('prefixes the path, sends the auth header and returns parsed JSON', async () => {
		fetchMock.mockResolvedValue(jsonResponse(200, { apps: [] }));

		await expect(authenticatedApiCall('apps/list')).resolves.toEqual({ apps: [] });

		const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit];
		expect(url).toBe('/api/v1/authenticated/apps/list');
		expect(init.credentials).toBe('include');
		expect(init.headers).toMatchObject({ Authorization: 'Bearer test-token' });
	});

	it('throws the server message on a JSON error response', async () => {
		fetchMock.mockResolvedValue(
			jsonResponse(403, { error: true, message: 'Access denied: nope' }, 'Forbidden')
		);

		await expect(authenticatedApiCall('apps/rebuild/x')).rejects.toThrow('Access denied: nope');
	});

	it('throws with status and status text when the error body is not JSON', async () => {
		fetchMock.mockResolvedValue(
			new Response('<html>', { status: 403, statusText: 'Forbidden' })
		);

		await expect(authenticatedApiCall('apps/rebuild/x')).rejects.toThrow(
			'API call failed: 403 Forbidden'
		);
	});

	it('clears the session and redirects to login on 401', async () => {
		fetchMock.mockResolvedValue(jsonResponse(401, {}));

		await expect(authenticatedApiCall('apps/list')).rejects.toThrow('Unauthorized');
		expect(sessionStore.clearInvalidSession).toHaveBeenCalledOnce();
		expect(authService.logout).toHaveBeenCalledWith('/login');
	});
});
