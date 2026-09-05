import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { Writable } from 'svelte/store';

vi.mock('./userStore', async () => {
	const { writable } = await import('svelte/store');
	return { authMode: writable('oauth'), isLoggedIn: writable(true) };
});
vi.mock('$lib', () => ({ authenticatedApiCall: vi.fn() }));

import { authenticatedApiCall } from '$lib';
import { authMode as mockedAuthMode } from './userStore';
import {
	clearPermissionCache,
	getAppPermissions,
	hasAdminPermission,
	hasPermission,
	loadUserPermissions
} from './permissionStore';

const authMode = mockedAuthMode as unknown as Writable<'dev' | 'oauth' | 'bearer'>;

async function loadScopes(scopes: { name: string; permissions: string[] }[]) {
	vi.mocked(authenticatedApiCall).mockResolvedValue({
		scopes: scopes.map((s) => ({ description: '', ...s }))
	});
	await loadUserPermissions();
}

describe('permissionStore', () => {
	beforeEach(() => {
		authMode.set('oauth');
		clearPermissionCache();
		vi.spyOn(console, 'log').mockImplementation(() => {});
	});

	it('grants everything in dev auth mode', () => {
		authMode.set('dev');
		expect(hasPermission('any', 'destroy')).toBe(true);
	});

	it('grants a permission listed in one of the user scopes', async () => {
		await loadScopes([{ name: 'client-a', permissions: ['view', 'manage'] }]);
		expect(hasPermission('app', 'manage')).toBe(true);
		expect(hasPermission('app', 'destroy')).toBe(false);
	});

	it('treats a wildcard permission as all permissions', async () => {
		await loadScopes([{ name: 'default', permissions: ['*'] }]);
		expect(hasPermission('app', 'shell')).toBe(true);
		expect(hasAdminPermission()).toBe(true);
	});

	it('batches permission lookups', async () => {
		await loadScopes([{ name: 'default', permissions: ['view'] }]);
		expect(getAppPermissions('app', ['view', 'manage'])).toEqual({ view: true, manage: false });
	});

	it('denies everything when loading scopes fails', async () => {
		vi.mocked(authenticatedApiCall).mockRejectedValue(new Error('boom'));
		vi.spyOn(console, 'error').mockImplementation(() => {});
		await loadUserPermissions();
		expect(hasPermission('app', 'view')).toBe(false);
	});
});
