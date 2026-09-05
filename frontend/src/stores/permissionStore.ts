import { writable, derived, get } from 'svelte/store';
import { authMode, isLoggedIn } from './userStore';
import { apps } from './appsStore';
import type { App } from '../types';
import { authenticatedApiCall } from '$lib';

export type Permission =
	| 'view'
	| 'manage'
	| 'shell'
	| 'logs'
	| 'create'
	| 'destroy'
	| 'admin_read'
	| 'admin_write';

interface ScopeInfo {
	name: string;
	description: string;
	permissions: string[];
}

// Store for user's scopes and their permissions
const userScopes = writable<ScopeInfo[]>([]);

// Loading state
const permissionsLoading = writable<boolean>(false);
const permissionsLoadAttempted = writable<boolean>(false);
export { permissionsLoading };

/**
 * Load user's permissions and app scope mappings
 */
export async function loadUserPermissions(): Promise<void> {
	if (get(permissionsLoading)) return; // Prevent duplicate loading

	permissionsLoading.set(true);
	permissionsLoadAttempted.set(true);

	try {
		const response = (await authenticatedApiCall('scopes/list')) as { scopes: ScopeInfo[] };
		userScopes.set(response.scopes);
	} catch (error) {
		console.error('Error loading user permissions:', error);
		userScopes.set([]);
	} finally {
		permissionsLoading.set(false);
	}
}

/**
 * Check if user has a specific permission for an app.
 *
 * Grants are resolved from the scopes the app belongs to (apps without
 * settings live in `default`). Pass the `App` itself where you have it; a
 * name is looked up in the loaded app list. Unknown names and the `_global`
 * pseudo-app fall back to "any scope grants it".
 */
export function hasPermission(appOrName: App | string, permission: Permission): boolean {
	// In development mode, allow everything
	if (get(authMode) === 'dev') {
		return true;
	}

	let scopes = get(userScopes);
	const app =
		typeof appOrName === 'string' ? get(apps).find((a) => a.name === appOrName) : appOrName;
	if (app) {
		const appScopes = app.settings?.scopes ?? ['default'];
		scopes = scopes.filter((scope) => appScopes.includes(scope.name));
	}

	return scopes.some(
		(scope) => scope.permissions.includes(permission) || scope.permissions.includes('*')
	);
}

/**
 * Check if user has admin permissions
 */
export function hasAdminPermission(): boolean {
	return hasPermission('_global', 'admin_read') || hasPermission('_global', 'admin_write');
}

/**
 * Get all permissions for an app (batch operation)
 */
export function getAppPermissions(
	app: App | string,
	permissions: Permission[]
): Record<string, boolean> {
	const results: Record<string, boolean> = {};

	permissions.forEach((permission) => {
		results[permission] = hasPermission(app, permission);
	});

	return results;
}

/**
 * Get user's effective permissions (all permissions across all scopes)
 */
export function getUserEffectivePermissions(): Permission[] {
	const scopes = get(userScopes);
	const allPermissions = new Set<Permission>();

	scopes.forEach((scope) => {
		scope.permissions.forEach((perm) => {
			if (perm === '*') {
				// Add all permissions if wildcard
				[
					'view',
					'manage',
					'shell',
					'logs',
					'create',
					'destroy',
					'admin_read',
					'admin_write'
				].forEach((p) => allPermissions.add(p as Permission));
			} else {
				allPermissions.add(perm as Permission);
			}
		});
	});

	return Array.from(allPermissions);
}

/**
 * Clear permission cache
 */
export function clearPermissionCache(): void {
	userScopes.set([]);
}

/**
 * Derived store for reactive access to user scopes
 */
export const permissions = derived([userScopes, isLoggedIn], ([$userScopes, $isLoggedIn]) => {
	if (!$isLoggedIn) return [];
	return $userScopes;
});

/**
 * Derived store for loading state
 */
export const permissionsLoaded = derived(
	[userScopes, permissionsLoading, permissionsLoadAttempted],
	([, $loading, $attempted]) => !$loading && $attempted
);
