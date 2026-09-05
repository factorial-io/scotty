import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { Writable } from 'svelte/store';

vi.mock('./userStore', async () => {
	const { writable } = await import('svelte/store');
	return { authMode: writable('oauth'), isLoggedIn: writable(true) };
});
vi.mock('$lib', () => ({ authenticatedApiCall: vi.fn() }));

import { authenticatedApiCall } from '$lib';
import { authMode as mockedAuthMode } from './userStore';
import { apps } from './appsStore';
import type { App } from '../types';
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

function setApp(name: string, scopes: string[] | null) {
	const settings = scopes === null ? null : { scopes };
	apps.set([{ name, settings } as unknown as App]);
}

describe('permissionStore', () => {
	beforeEach(() => {
		authMode.set('oauth');
		clearPermissionCache();
		apps.set([]);
	});

	it('grants everything in dev auth mode', () => {
		authMode.set('dev');
		expect(hasPermission('any', 'destroy')).toBe(true);
	});

	it('grants a permission listed in one of the user scopes for an unknown app', async () => {
		await loadScopes([{ name: 'client-a', permissions: ['view', 'manage'] }]);
		expect(hasPermission('app', 'manage')).toBe(true);
		expect(hasPermission('app', 'destroy')).toBe(false);
	});

	it('denies a permission the user only holds in another scope', async () => {
		await loadScopes([
			{ name: 'client-a', permissions: ['view', 'manage'] },
			{ name: 'default', permissions: ['view'] }
		]);
		setApp('circle-dot', ['default']);
		expect(hasPermission('circle-dot', 'view')).toBe(true);
		expect(hasPermission('circle-dot', 'manage')).toBe(false);
	});

	it('grants a permission held in one of the app scopes', async () => {
		await loadScopes([{ name: 'client-a', permissions: ['view', 'manage'] }]);
		setApp('app', ['client-a', 'client-b']);
		expect(hasPermission('app', 'manage')).toBe(true);
	});

	it('treats an app without settings as belonging to default', async () => {
		await loadScopes([{ name: 'default', permissions: ['view'] }]);
		setApp('app', null);
		expect(hasPermission('app', 'view')).toBe(true);
		expect(hasPermission('app', 'manage')).toBe(false);
	});

	it('resolves scopes from a passed App even when the app list is empty', async () => {
		await loadScopes([{ name: 'client-a', permissions: ['view', 'manage'] }]);
		const app = { name: 'circle-dot', settings: { scopes: ['default'] } } as unknown as App;
		expect(hasPermission(app, 'manage')).toBe(false);
		expect(getAppPermissions(app, ['manage'])).toEqual({ manage: false });
	});

	it('resolves _global against any scope', async () => {
		await loadScopes([{ name: 'client-a', permissions: ['admin_read'] }]);
		setApp('app', ['default']);
		expect(hasAdminPermission()).toBe(true);
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
