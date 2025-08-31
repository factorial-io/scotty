import { writable, derived, get } from 'svelte/store';
import { authMode, isLoggedIn } from './userStore';
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

interface AppScopeMapping {
	[appName: string]: string[]; // app -> scopes it belongs to
}

// Store for user's scopes and their permissions
const userScopes = writable<ScopeInfo[]>([]);

// Store for app to scope mappings (we'll need to fetch this)
const appScopes = writable<AppScopeMapping>({});

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
		// Load user scopes with permissions
		console.log('Loading user permissions from scopes/list endpoint...');
		const scopesResponse = await authenticatedApiCall('scopes/list');
		const response = scopesResponse as { scopes: ScopeInfo[] };
		console.log('Received scopes:', response);
		userScopes.set(response.scopes);

		// For now, we don't have an endpoint that gives us app->scope mappings
		// Apps are filtered by backend, so we assume user can see apps they have permissions for
		// This is a simplification - in a full implementation, you'd want this mapping
	} catch (error) {
		console.error('Error loading user permissions:', error);
		userScopes.set([]);
	} finally {
		permissionsLoading.set(false);
	}
}

/**
 * Check if user has a specific permission for an app
 * This is now synchronous and uses cached data
 */
export function hasPermission(appName: string, permission: Permission): boolean {
	// In development mode, allow everything
	const currentAuthMode = get(authMode);
	if (currentAuthMode === 'dev') {
		return true;
	}

	const scopes = get(userScopes);

	// Check if user has this permission in any of their scopes
	// Since we don't have app->scope mapping yet, we check all user scopes
	// This is permissive - if user has the permission in any scope, they can use it
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
	appName: string,
	permissions: Permission[]
): Record<string, boolean> {
	const results: Record<string, boolean> = {};

	permissions.forEach((permission) => {
		results[permission] = hasPermission(appName, permission);
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
	appScopes.set({});
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
	([$userScopes, $loading, $attempted]) => !$loading && $attempted
);
