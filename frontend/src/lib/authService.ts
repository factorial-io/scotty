/**
 * AuthService - Centralized authentication flow management
 *
 * This service handles all authentication operations including login, OAuth,
 * logout, and token validation. It integrates with the SessionStore to provide
 * a unified authentication experience.
 */

import { sessionStore } from '../stores/sessionStore';
import { goto } from '$app/navigation';
import { resolve } from '$app/paths';
import type { UserInfo } from '../types';

export interface LoginResult {
	success: boolean;
	error?: string;
	redirectUrl?: string;
}

export interface TokenResponse {
	access_token: string;
	user_id: string;
	user_name: string;
	user_email: string;
	user_picture?: string;
}

export interface OAuthErrorResponse {
	error: string;
	error_description?: string;
}

/**
 * AuthService provides centralized authentication operations
 */
export class AuthService {
	/**
	 * Initialize authentication system
	 */
	async initialize(): Promise<void> {
		await sessionStore.init();
	}

	/**
	 * Handle bearer token login
	 */
	async loginWithPassword(password: string): Promise<LoginResult> {
		try {
			const response = await fetch('/api/v1/login', {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json'
				},
				body: JSON.stringify({ password }),
				credentials: 'include'
			});

			if (!response.ok) {
				return { success: false, error: 'Invalid credentials' };
			}

			const result = await response.json();

			if (result.status === 'success') {
				if (result.token) {
					sessionStore.setBearerToken(result.token);
				}
				return { success: true };
			} else if (result.status === 'redirect') {
				return { success: true, redirectUrl: result.redirect_url };
			} else {
				return { success: false, error: result.message || 'Login failed' };
			}
		} catch (error) {
			console.error('Login error:', error);
			return { success: false, error: 'Network error occurred' };
		}
	}

	/**
	 * Initiate OAuth flow
	 */
	initiateOAuthFlow(): void {
		window.location.href = '/oauth/authorize';
	}

	/**
	 * Handle OAuth callback
	 */
	async handleOAuthCallback(sessionId: string): Promise<LoginResult> {
		try {
			// Exchange session for token
			const response = await fetch('/oauth/exchange', {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json'
				},
				body: JSON.stringify({
					session_id: sessionId
				})
			});

			if (!response.ok) {
				const errorData: OAuthErrorResponse = await response.json();
				return {
					success: false,
					error: `Authentication failed: ${errorData.error_description || errorData.error}`
				};
			}

			const tokenData: TokenResponse = await response.json();

			// Create user info object
			const userInfo: UserInfo = {
				id: tokenData.user_id,
				name: tokenData.user_name,
				email: tokenData.user_email,
				picture: tokenData.user_picture
			};

			// Store OAuth session
			sessionStore.setOAuthSession(tokenData.access_token, userInfo);

			return { success: true };
		} catch (error) {
			console.error('OAuth callback error:', error);
			return {
				success: false,
				error: error instanceof Error ? error.message : 'An unexpected error occurred'
			};
		}
	}

	/**
	 * Logout user and redirect
	 */
	async logout(redirectTo: string = '/login'): Promise<void> {
		sessionStore.logout();
		await goto(resolve(redirectTo));
	}

	/**
	 * Check if user needs authentication
	 */
	async requireAuth(currentPath: string): Promise<boolean> {
		const authMode = sessionStore.getAuthMode();

		// Skip auth check in development mode
		if (authMode === 'dev') {
			return true;
		}

		// Skip auth check on login and OAuth pages
		if (currentPath === '/login' || currentPath.startsWith('/oauth/')) {
			return true;
		}

		// Check if user is authenticated
		if (!sessionStore.isAuthenticated()) {
			await this.logout('/login');
			return false;
		}

		// Validate current token
		const isValid = await sessionStore.validateCurrentToken();
		if (!isValid) {
			await this.logout('/login');
			return false;
		}

		return true;
	}

	/**
	 * Get auth mode from current session
	 */
	getAuthMode(): string {
		return sessionStore.getAuthMode();
	}

	/**
	 * Check if user is authenticated
	 */
	isAuthenticated(): boolean {
		return sessionStore.isAuthenticated();
	}

	/**
	 * Get current user info
	 */
	getUserInfo(): UserInfo | null {
		return sessionStore.getUserInfo();
	}

	/**
	 * Get authorization header for API requests
	 */
	getAuthHeader(): Record<string, string> {
		return sessionStore.getAuthHeader();
	}

	/**
	 * Validate stored token
	 */
	async validateToken(): Promise<boolean> {
		return await sessionStore.validateCurrentToken();
	}

	/**
	 * Check auth mode and handle initial authentication check
	 */
	async checkAuthMode(): Promise<{ authMode: string; message?: string }> {
		try {
			// First validate any existing token
			if (sessionStore.isAuthenticated()) {
				const isValid = await sessionStore.validateCurrentToken();
				if (isValid) {
					// Already authenticated, can redirect to dashboard
					return { authMode: sessionStore.getAuthMode() };
				}
			}

			// Not authenticated, get auth mode info
			const response = await fetch('/api/v1/login', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ password: '' }),
				credentials: 'include'
			});

			if (response.ok) {
				const result = await response.json();
				return {
					authMode: result.auth_mode || 'bearer',
					message: result.message || ''
				};
			}

			// Fallback to bearer mode
			return { authMode: 'bearer' };
		} catch (error) {
			console.warn('Failed to check auth mode:', error);
			return { authMode: 'bearer' };
		}
	}
}

// Singleton instance
export const authService = new AuthService();
