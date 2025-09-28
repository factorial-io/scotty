/**
 * UserStore - Simple re-exports from SessionStore
 *
 * This module provides clean re-exports of SessionStore functionality
 * for components that expect user-related stores.
 */

// Re-export all SessionStore functionality
export {
	sessionStore,
	authMode,
	userInfo,
	isAuthenticated,
	isInitialized,
	currentTokenType
} from './sessionStore';

// Legacy alias for backward compatibility
export { isAuthenticated as isLoggedIn } from './sessionStore';

// Re-export types
export type { UserInfo } from '../types';

// Legacy export for any components still expecting authStore
export { sessionStore as authStore } from './sessionStore';