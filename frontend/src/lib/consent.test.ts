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
import { canApprove, decideConsent, describeScope, loadConsent, selectedScopes } from './consent';

const api = vi.mocked(authenticatedApiCall);
const requested = ['read', 'create', 'destroy', 'offline_access'];

describe('consent page logic', () => {
	beforeEach(() => api.mockReset());

	it('drops unticked scopes from the approve body', async () => {
		const ticked = { read: true, create: true, destroy: false, offline_access: true };
		const scopes = selectedScopes(requested, ticked);
		expect(scopes).toEqual(['read', 'create', 'offline_access']);

		api.mockResolvedValue({ redirect: 'http://127.0.0.1:4567/cb?code=x' });
		await expect(decideConsent('abc', scopes)).resolves.toEqual({
			kind: 'redirect',
			url: 'http://127.0.0.1:4567/cb?code=x'
		});
		expect(api).toHaveBeenCalledWith('oauth/consent/abc/approve', {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ scopes: ['read', 'create', 'offline_access'] })
		});
	});

	it('cannot approve with no scope ticked', () => {
		expect(canApprove(requested, {})).toBe(false);
		expect(canApprove(requested, { read: false })).toBe(false);
		expect(canApprove(requested, { destroy: true })).toBe(true);
	});

	it('denies and returns the redirect', async () => {
		api.mockResolvedValue({ redirect: 'https://client.test/cb?error=access_denied' });
		await expect(decideConsent('abc', null)).resolves.toEqual({
			kind: 'redirect',
			url: 'https://client.test/cb?error=access_denied'
		});
		expect(api).toHaveBeenCalledWith('oauth/consent/abc/deny', { method: 'POST' });
	});

	it('loads the request', async () => {
		const request = {
			id: 'abc',
			client_id: 'c1',
			client_name: 'Claude',
			redirect_host: 'claude.ai',
			loopback_only: false,
			scopes: requested
		};
		api.mockResolvedValue(request);
		await expect(loadConsent('abc')).resolves.toEqual({ kind: 'pending', request });
		expect(api).toHaveBeenCalledWith('oauth/consent/abc');
	});

	it('maps 404 to invalid, other errors to error', async () => {
		api.mockRejectedValueOnce(new ApiError('gone', 404));
		await expect(loadConsent('abc')).resolves.toEqual({ kind: 'invalid' });

		api.mockRejectedValueOnce(new ApiError('gone', 404));
		await expect(decideConsent('abc', ['read'])).resolves.toEqual({ kind: 'invalid' });

		api.mockRejectedValueOnce(new ApiError('bad scopes', 400));
		await expect(decideConsent('abc', ['read'])).resolves.toEqual({
			kind: 'error',
			message: 'bad scopes'
		});
	});

	it('describes known scopes in plain words', () => {
		expect(describeScope('destroy')).toBe('Destroy apps');
		expect(describeScope('custom')).toBe('custom');
	});
});
