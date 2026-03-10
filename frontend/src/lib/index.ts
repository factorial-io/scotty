/**
 * API Utilities - Simplified authentication with SessionStore integration
 *
 * This module provides a unified API interface that integrates with the new
 * SessionStore for authentication. It simplifies the previous complex logic
 * by delegating authentication concerns to the centralized session management.
 */

import { sessionStore } from '../stores/sessionStore';
import { authService } from './authService';
import type { ServerInfo } from '../types';

export type AuthMode = 'dev' | 'oauth' | 'bearer';

/**
 * Make authenticated API calls using SessionStore
 */
export async function authenticatedApiCall(
	url: string,
	options: RequestInit = {}
): Promise<unknown> {
	// Ensure session is initialized
	if (!sessionStore.isAuthenticated()) {
		await sessionStore.init();
	}

	// Prepare request options
	const requestOptions: RequestInit = {
		...options,
		credentials: 'include', // Always include cookies for OAuth mode
		headers: {
			...options.headers,
			...sessionStore.getAuthHeader() // Get auth header from session store
		}
	};

	try {
		const response = await fetch(`/api/v1/authenticated/${url}`, requestOptions);

		// Handle 401 Unauthorized
		if (response.status === 401) {
			await handleUnauthorized();
			throw new Error('Unauthorized');
		}

		if (!response.ok) {
			throw new Error(`API call failed: ${response.status} ${response.statusText}`);
		}

		return await response.json();
	} catch (error) {
		console.error('API call failed:', error);
		throw error;
	}
}

/**
 * Make public API calls (no authentication required)
 */
export async function publicApiCall(url: string, options: RequestInit = {}): Promise<unknown> {
	try {
		const response = await fetch(`/api/v1/${url}`, options);

		if (!response.ok) {
			throw new Error(`API call failed: ${response.status} ${response.statusText}`);
		}

		return await response.json();
	} catch (error) {
		console.error('Public API call failed:', error);
		throw error;
	}
}

/**
 * Legacy function for backward compatibility - will be deprecated
 */
export async function apiCall(url: string, options: RequestInit = {}): Promise<unknown> {
	console.warn('apiCall is deprecated. Use authenticatedApiCall or publicApiCall instead.');
	return authenticatedApiCall(url, options);
}

/**
 * Handle unauthorized responses by clearing session and redirecting
 */
async function handleUnauthorized(): Promise<void> {
	console.warn('Unauthorized response received, clearing session and redirecting to login');

	// Clear invalid session
	await sessionStore.clearInvalidSession();

	// Don't redirect if already on login, OAuth, or landing pages
	if (typeof window !== 'undefined') {
		const currentPath = window.location.pathname;
		if (
			currentPath === '/login' ||
			currentPath.startsWith('/oauth/') ||
			currentPath.startsWith('/landing/')
		) {
			return;
		}

		// Redirect to login
		await authService.logout('/login');
	}
}

/**
 * Get current auth mode
 */
export async function getAuthMode(): Promise<AuthMode> {
	try {
		const info = await publicApiCall('info');
		return (info as ServerInfo).auth_mode || 'bearer';
	} catch (error) {
		console.warn('Failed to get auth mode, defaulting to bearer:', error);
		return 'bearer';
	}
}

/**
 * Validate current token
 */
export async function validateToken(token?: string): Promise<boolean> {
	if (token) {
		// Validate specific token
		try {
			const response = await fetch('/api/v1/authenticated/validate-token', {
				method: 'POST',
				headers: {
					Authorization: `Bearer ${token}`
				},
				credentials: 'include'
			});
			return response.ok;
		} catch (error) {
			console.warn('Token validation failed:', error);
			return false;
		}
	} else {
		// Validate current session token
		return await sessionStore.validateCurrentToken();
	}
}

/**
 * Initialize authentication and redirect if needed
 */
export async function initializeAuth(): Promise<void> {
	await authService.initialize();

	// Check if current page requires authentication
	if (typeof window !== 'undefined') {
		const currentPath = window.location.pathname;
		await authService.requireAuth(currentPath);
	}
}

/**
 * Check if user is authenticated
 */
export function isAuthenticated(): boolean {
	return sessionStore.isAuthenticated();
}

/**
 * Get current user info
 */
export function getCurrentUser() {
	return sessionStore.getUserInfo();
}

/**
 * Get authorization header for manual requests
 */
export function getAuthHeader(): Record<string, string> {
	return sessionStore.getAuthHeader();
}

// getAuthMode is already exported above
