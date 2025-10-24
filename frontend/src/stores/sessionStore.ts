/**
 * SessionStore - Reactive Svelte store for authentication session management
 *
 * This store provides a reactive interface for managing authentication tokens,
 * user information, and session state. It integrates with the SessionManager
 * for storage operations and provides real-time updates to components.
 */

import { writable, derived, get } from 'svelte/store';
import { sessionManager, type AuthMode, type TokenType } from '../lib/sessionManager';
import type { UserInfo } from '../types';

export interface SessionState {
	authMode: AuthMode;
	userInfo: UserInfo | null;
	isAuthenticated: boolean;
	isInitialized: boolean;
	currentTokenType: TokenType | null;
}

/**
 * Create the reactive session store
 */
function createSessionStore() {
	const { subscribe, set, update } = writable<SessionState>({
		authMode: 'bearer',
		userInfo: null,
		isAuthenticated: false,
		isInitialized: false,
		currentTokenType: null
	});

	return {
		subscribe,

		/**
		 * Initialize session store from storage and API
		 */
		async init(): Promise<void> {
			try {
				// Get auth mode from API
				const authMode = await this.fetchAuthMode();

				// Load session data from storage
				const userInfo = sessionManager.getUserInfo();
				const tokenData = sessionManager.getAnyToken();

				// Determine authentication status
				let isAuthenticated = false;
				let currentTokenType: TokenType | null = null;

				if (authMode === 'dev') {
					isAuthenticated = true; // Always authenticated in dev mode
				} else if (tokenData) {
					currentTokenType = tokenData.type;
					isAuthenticated = await this.validateCurrentToken();
				}

				// Update store state
				set({
					authMode,
					userInfo,
					isAuthenticated,
					isInitialized: true,
					currentTokenType
				});

				console.debug('Session store initialized:', {
					authMode,
					isAuthenticated,
					hasUserInfo: !!userInfo,
					tokenType: currentTokenType
				});
			} catch (error) {
				console.error('Failed to initialize session store:', error);
				set({
					authMode: 'bearer',
					userInfo: null,
					isAuthenticated: false,
					isInitialized: true,
					currentTokenType: null
				});
			}
		},

		/**
		 * Set bearer token and update session
		 */
		setBearerToken(token: string): void {
			if (sessionManager.setBearerToken(token)) {
				update((state) => ({
					...state,
					isAuthenticated: true,
					currentTokenType: 'bearer'
				}));
			}
		},

		/**
		 * Set OAuth token and user info
		 */
		setOAuthSession(token: string, userInfo: UserInfo): void {
			const tokenSet = sessionManager.setOAuthToken(token);
			const userInfoSet = sessionManager.setUserInfo(userInfo);

			if (tokenSet && userInfoSet) {
				// Clear any existing bearer token
				sessionManager.removeBearerToken();

				update((state) => ({
					...state,
					userInfo,
					isAuthenticated: true,
					currentTokenType: 'oauth'
				}));
			}
		},

		/**
		 * Get current authentication token for API calls
		 */
		getAuthToken(): string | null {
			const tokenData = sessionManager.getAnyToken();
			return tokenData?.token || null;
		},

		/**
		 * Get authorization header for API requests
		 */
		getAuthHeader(): Record<string, string> {
			const token = this.getAuthToken();
			return token ? { Authorization: `Bearer ${token}` } : {};
		},

		/**
		 * Validate current token with server
		 */
		async validateCurrentToken(): Promise<boolean> {
			const token = this.getAuthToken();
			if (!token) return false;

			try {
				const response = await fetch('/api/v1/authenticated/validate-token', {
					method: 'POST',
					headers: {
						Authorization: `Bearer ${token}`
					},
					credentials: 'include'
				});

				if (!response.ok) {
					// Token is invalid, clear it
					await this.clearInvalidSession();
					return false;
				}

				return true;
			} catch (error) {
				console.warn('Token validation failed:', error);
				return false;
			}
		},

		/**
		 * Clear session and logout user
		 */
		logout(): void {
			sessionManager.clearAllTokens();

			update((state) => ({
				...state,
				userInfo: null,
				isAuthenticated: false,
				currentTokenType: null
			}));
		},

		/**
		 * Clear invalid session data
		 */
		async clearInvalidSession(): Promise<void> {
			const currentState = get({ subscribe });

			if (currentState.currentTokenType === 'oauth') {
				sessionManager.clearOAuthSession();
			} else if (currentState.currentTokenType === 'bearer') {
				sessionManager.clearBearerSession();
			}

			update((state) => ({
				...state,
				userInfo: null,
				isAuthenticated: false,
				currentTokenType: null
			}));
		},

		/**
		 * Fetch auth mode from server
		 */
		async fetchAuthMode(): Promise<AuthMode> {
			try {
				const response = await fetch('/api/v1/info');
				const info = await response.json();
				return info.auth_mode || 'bearer';
			} catch (error) {
				console.warn('Failed to fetch auth mode, defaulting to bearer:', error);
				return 'bearer';
			}
		},

		/**
		 * Check if user is authenticated
		 */
		isAuthenticated(): boolean {
			const state = get({ subscribe });
			return state.isAuthenticated;
		},

		/**
		 * Get current auth mode
		 */
		getAuthMode(): AuthMode {
			const state = get({ subscribe });
			return state.authMode;
		},

		/**
		 * Get user info
		 */
		getUserInfo(): UserInfo | null {
			const state = get({ subscribe });
			return state.userInfo;
		},

		/**
		 * Debug helper
		 */
		getDebugInfo() {
			const state = get({ subscribe });
			const storageInfo = sessionManager.getStorageDebugInfo();

			return {
				storeState: state,
				storageInfo
			};
		}
	};
}

// Create singleton instance
export const sessionStore = createSessionStore();

// Derived stores for convenient access
export const authMode = derived(sessionStore, ($session) => $session.authMode);
export const userInfo = derived(sessionStore, ($session) => $session.userInfo);
export const isAuthenticated = derived(sessionStore, ($session) => $session.isAuthenticated);
export const isInitialized = derived(sessionStore, ($session) => $session.isInitialized);
export const currentTokenType = derived(sessionStore, ($session) => $session.currentTokenType);
