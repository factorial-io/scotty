import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('$lib', async () => {
	class ApiError extends Error {
		constructor(
			message: string,
			public status: number
		) {
			super(message);
		}
	}
	return { ApiError, authenticatedApiCall: vi.fn() };
});

import { ApiError, authenticatedApiCall } from '$lib';
import {
	codeFromUrl,
	decideDeviceCode,
	lookupDeviceCode,
	normalizeUserCode,
	scopeList
} from './deviceAuth';

const api = vi.mocked(authenticatedApiCall);

describe('device page logic', () => {
	beforeEach(() => api.mockReset());

	it('prefills the code from the URL, normalized', () => {
		expect(codeFromUrl(new URL('https://scotty.test/device?code=abcd-efgh'))).toBe('ABCD-EFGH');
		expect(codeFromUrl(new URL('https://scotty.test/device'))).toBe('');
		expect(normalizeUserCode(' ab cd-efgh ')).toBe('ABCD-EFGH');
	});

	it('splits the scope string', () => {
		expect(scopeList('offline_access  apps:read')).toEqual(['offline_access', 'apps:read']);
		expect(scopeList('')).toEqual([]);
	});

	it('looks up a code and returns the pending request', async () => {
		const request = { user_code: 'ABCD-EFGH', client_name: 'scottyctl on host', scope: 'x' };
		api.mockResolvedValue(request);

		await expect(lookupDeviceCode('abcd-efgh')).resolves.toEqual({ kind: 'pending', request });
		expect(api).toHaveBeenCalledWith('oauth/device/ABCD-EFGH');
	});

	it('does not call the API for an empty code', async () => {
		await expect(lookupDeviceCode('  ')).resolves.toEqual({ kind: 'idle' });
		expect(api).not.toHaveBeenCalled();
	});

	it('maps 404 to invalid or expired, other errors to error', async () => {
		api.mockRejectedValueOnce(new ApiError('not found', 404));
		await expect(lookupDeviceCode('X')).resolves.toEqual({ kind: 'invalid' });

		api.mockRejectedValueOnce(new ApiError('boom', 500));
		await expect(lookupDeviceCode('X')).resolves.toEqual({ kind: 'error', message: 'boom' });

		api.mockRejectedValueOnce(new ApiError('gone', 404));
		await expect(decideDeviceCode('X', true)).resolves.toEqual({ kind: 'invalid' });
	});

	it('approves and denies with the user code', async () => {
		api.mockResolvedValue(null);

		await expect(decideDeviceCode('abcd-efgh', true)).resolves.toEqual({ kind: 'approved' });
		expect(api).toHaveBeenLastCalledWith('oauth/device/approve', {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ user_code: 'ABCD-EFGH' })
		});

		await expect(decideDeviceCode('abcd-efgh', false)).resolves.toEqual({ kind: 'denied' });
		expect(api.mock.lastCall?.[0]).toBe('oauth/device/deny');
	});
});
