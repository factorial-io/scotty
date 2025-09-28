/**
 * SessionManager - Centralized storage abstraction for authentication tokens and user data
 *
 * This module provides a unified interface for managing authentication-related data
 * in sessionStorage, with type-safe key management and proper error handling.
 */

import { browser } from '$app/environment';
import type { UserInfo } from '../types';

export type AuthMode = 'dev' | 'oauth' | 'bearer';
export type TokenType = 'bearer' | 'oauth';

// Storage keys - centralized to avoid typos and enable easy refactoring
const STORAGE_KEYS = {
	BEARER_TOKEN: 'token',
	OAUTH_TOKEN: 'oauth_token',
	USER_INFO: 'user_info',
	AUTH_MODE: 'auth_mode'
} as const;

/**
 * SessionManager handles all sessionStorage operations with proper error handling
 */
export class SessionManager {
	private isAvailable(): boolean {
		return browser && typeof sessionStorage !== 'undefined';
	}

	private getItem(key: string): string | null {
		if (!this.isAvailable()) return null;

		try {
			return sessionStorage.getItem(key);
		} catch (error) {
			console.warn(`Failed to read from sessionStorage (${key}):`, error);
			return null;
		}
	}

	private setItem(key: string, value: string): boolean {
		if (!this.isAvailable()) return false;

		try {
			sessionStorage.setItem(key, value);
			return true;
		} catch (error) {
			console.warn(`Failed to write to sessionStorage (${key}):`, error);
			return false;
		}
	}

	private removeItem(key: string): boolean {
		if (!this.isAvailable()) return false;

		try {
			sessionStorage.removeItem(key);
			return true;
		} catch (error) {
			console.warn(`Failed to remove from sessionStorage (${key}):`, error);
			return false;
		}
	}

	// Token Management
	getBearerToken(): string | null {
		return this.getItem(STORAGE_KEYS.BEARER_TOKEN);
	}

	setBearerToken(token: string): boolean {
		return this.setItem(STORAGE_KEYS.BEARER_TOKEN, token);
	}

	removeBearerToken(): boolean {
		return this.removeItem(STORAGE_KEYS.BEARER_TOKEN);
	}

	getOAuthToken(): string | null {
		return this.getItem(STORAGE_KEYS.OAUTH_TOKEN);
	}

	setOAuthToken(token: string): boolean {
		return this.setItem(STORAGE_KEYS.OAUTH_TOKEN, token);
	}

	removeOAuthToken(): boolean {
		return this.removeItem(STORAGE_KEYS.OAUTH_TOKEN);
	}

	// Get any available token (OAuth takes precedence)
	getAnyToken(): { token: string; type: TokenType } | null {
		const oauthToken = this.getOAuthToken();
		if (oauthToken) {
			return { token: oauthToken, type: 'oauth' };
		}

		const bearerToken = this.getBearerToken();
		if (bearerToken) {
			return { token: bearerToken, type: 'bearer' };
		}

		return null;
	}

	// User Info Management
	getUserInfo(): UserInfo | null {
		const userInfoJson = this.getItem(STORAGE_KEYS.USER_INFO);
		if (!userInfoJson) return null;

		try {
			return JSON.parse(userInfoJson);
		} catch (error) {
			console.warn('Failed to parse user info from sessionStorage:', error);
			// Clean up invalid data
			this.removeUserInfo();
			return null;
		}
	}

	setUserInfo(userInfo: UserInfo): boolean {
		try {
			const userInfoJson = JSON.stringify(userInfo);
			return this.setItem(STORAGE_KEYS.USER_INFO, userInfoJson);
		} catch (error) {
			console.warn('Failed to serialize user info:', error);
			return false;
		}
	}

	removeUserInfo(): boolean {
		return this.removeItem(STORAGE_KEYS.USER_INFO);
	}

	// Session Operations
	clearAllTokens(): void {
		this.removeBearerToken();
		this.removeOAuthToken();
		this.removeUserInfo();
	}

	clearOAuthSession(): void {
		this.removeOAuthToken();
		this.removeUserInfo();
	}

	clearBearerSession(): void {
		this.removeBearerToken();
	}

	// Utility method to check if any authentication data exists
	hasAuthenticationData(): boolean {
		return this.getBearerToken() !== null || this.getOAuthToken() !== null;
	}

	// Debug helper
	getStorageDebugInfo(): Record<string, string | null> {
		if (!this.isAvailable()) {
			return { error: 'sessionStorage not available' };
		}

		return {
			bearerToken: this.getBearerToken() ? 'present' : 'none',
			oauthToken: this.getOAuthToken() ? 'present' : 'none',
			userInfo: this.getUserInfo() ? 'present' : 'none',
			totalKeys: Object.keys(sessionStorage).length.toString()
		};
	}
}

// Singleton instance
export const sessionManager = new SessionManager();