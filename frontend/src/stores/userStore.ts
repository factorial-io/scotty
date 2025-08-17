import { writable, derived } from 'svelte/store';
import { browser } from '$app/environment';

export interface UserInfo {
	id: string;
	name: string;
	email: string;
}

export interface AuthState {
	authMode: 'dev' | 'oauth' | 'bearer';
	userInfo: UserInfo | null;
	isLoggedIn: boolean;
}

// Create the auth store
function createAuthStore() {
	const { subscribe, set, update } = writable<AuthState>({
		authMode: 'bearer',
		userInfo: null,
		isLoggedIn: false
	});

	return {
		subscribe,
		set,
		update,
		// Initialize the store from localStorage and API
		init: async () => {
			if (!browser) return;

			try {
				// Get auth mode from API
				const response = await fetch('/api/v1/info');
				const info = await response.json();
				const authMode = info.auth_mode || 'bearer';

				let userInfo: UserInfo | null = null;
				let isLoggedIn = false;

				if (authMode === 'oauth') {
					// Try to get user info from localStorage
					const userInfoJson = localStorage.getItem('user_info');
					const token = localStorage.getItem('oauth_token');
					if (userInfoJson && token) {
						try {
							userInfo = JSON.parse(userInfoJson);
							isLoggedIn = true;
						} catch (error) {
							console.error('Failed to parse user info:', error);
							// Clear invalid data
							localStorage.removeItem('user_info');
							localStorage.removeItem('oauth_token');
						}
					}
				} else if (authMode === 'bearer') {
					const token = localStorage.getItem('token');
					isLoggedIn = !!token;
				} else if (authMode === 'dev') {
					isLoggedIn = true; // Always logged in during dev mode
				}

				set({ authMode, userInfo, isLoggedIn });
			} catch (error) {
				console.error('Failed to initialize auth store:', error);
				set({ authMode: 'bearer', userInfo: null, isLoggedIn: false });
			}
		},
		// Set user info after successful OAuth login
		setUserInfo: (userInfo: UserInfo) => {
			update((state) => ({ ...state, userInfo, isLoggedIn: true }));
		},
		// Clear user info on logout
		logout: () => {
			update((state) => ({ ...state, userInfo: null, isLoggedIn: false }));
		}
	};
}

export const authStore = createAuthStore();

// Derived store for easy access to just user info
export const userInfo = derived(authStore, ($authStore) => $authStore.userInfo);

// Derived store for auth mode
export const authMode = derived(authStore, ($authStore) => $authStore.authMode);

// Derived store for login status
export const isLoggedIn = derived(authStore, ($authStore) => $authStore.isLoggedIn);
