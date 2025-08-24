// place files you want to import through the `$lib` alias in this folder.

type AuthMode = 'dev' | 'oauth' | 'bearer';

// Cache auth mode to avoid repeated requests
let authMode: AuthMode | null = null;

// Get auth mode from server (cached after first call)
async function getAuthMode(): Promise<AuthMode> {
	if (authMode) {
		return authMode;
	}

	try {
		const result = (await publicApiCall('info')) as { auth_mode?: AuthMode };
		authMode = result.auth_mode || 'bearer';
	} catch (error) {
		console.warn('Failed to detect auth mode, defaulting to bearer:', error);
		authMode = 'bearer';
	}

	return authMode;
}

// Public API calls (health, info, login) - no authentication required
export async function publicApiCall(url: string, options: RequestInit = {}): Promise<unknown> {
	if (typeof window !== 'undefined') {
		options.credentials = 'include'; // Always include cookies
		const response = await fetch(`/api/v1/${url}`, options);
		const result = await response.json();
		return result;
	}
}

// Authenticated API calls (apps, tasks, etc.) - requires authentication
export async function authenticatedApiCall(
	url: string,
	options: RequestInit = {}
): Promise<unknown> {
	if (typeof window !== 'undefined') {
		const mode = await getAuthMode();

		// Always include cookies for OAuth mode
		options.credentials = 'include';

		// Add bearer token based on auth mode
		if (mode === 'bearer') {
			const currentToken = localStorage.getItem('token');
			if (currentToken) {
				options.headers = {
					...options.headers,
					Authorization: `Bearer ${currentToken}`
				};
			}
		} else if (mode === 'oauth') {
			const oauthToken = localStorage.getItem('oauth_token');
			if (oauthToken) {
				options.headers = {
					...options.headers,
					Authorization: `Bearer ${oauthToken}`
				};
			}
		}

		const response = await fetch(`/api/v1/authenticated/${url}`, options);

		// Handle 401 Unauthorized based on auth mode
		if (response.status === 401) {
			handleUnauthorized(mode);
			return Promise.reject(new Error('Unauthorized'));
		}

		const result = await response.json();
		return result;
	}
}

// Legacy function for backward compatibility - will be deprecated
export async function apiCall(url: string, options: RequestInit = {}): Promise<unknown> {
	console.warn('apiCall is deprecated. Use authenticatedApiCall or publicApiCall instead.');
	return authenticatedApiCall(url, options);
}

function handleUnauthorized(mode: AuthMode) {
	if (window.location.pathname === '/login' || window.location.pathname.startsWith('/oauth/')) {
		return; // Already on login page or OAuth flow
	}

	switch (mode) {
		case 'oauth':
			// Clear OAuth tokens and redirect to login
			localStorage.removeItem('oauth_token');
			localStorage.removeItem('user_info');
			window.location.href = '/login';
			break;
		case 'bearer':
			window.location.href = '/login';
			break;
		case 'dev':
			// In dev mode, 401 shouldn't happen, but refresh the page
			console.warn('Unexpected 401 in development mode');
			window.location.reload();
			break;
	}
}

export async function validateToken(token: string) {
	const response = await fetch('/api/v1/authenticated/validate-token', {
		method: 'POST',
		headers: {
			Authorization: `Bearer ${token}`
		},
		credentials: 'include'
	});

	if (
		!response.ok &&
		window.location.pathname !== '/login' &&
		!window.location.pathname.startsWith('/oauth/')
	) {
		const mode = await getAuthMode();
		handleUnauthorized(mode);
	}
}

export async function checkIfLoggedIn() {
	const mode = await getAuthMode();

	// Skip auth check in development mode
	if (mode === 'dev') {
		return;
	}

	// For OAuth mode, check for stored OAuth token
	if (mode === 'oauth') {
		const oauthToken = localStorage.getItem('oauth_token');
		if (
			!oauthToken &&
			window.location.pathname !== '/login' &&
			!window.location.pathname.startsWith('/oauth/')
		) {
			window.location.href = '/login';
			return;
		}

		if (oauthToken) {
			// Validate OAuth token
			try {
				await fetch('/api/v1/authenticated/validate-token', {
					method: 'POST',
					headers: {
						Authorization: `Bearer ${oauthToken}`
					},
					credentials: 'include'
				});
			} catch (error) {
				// If validate-token fails, clear token and redirect
				console.warn('OAuth token validation failed:', error);
				localStorage.removeItem('oauth_token');
				localStorage.removeItem('user_info');
				if (
					window.location.pathname !== '/login' &&
					!window.location.pathname.startsWith('/oauth/')
				) {
					window.location.href = '/login';
				}
			}
		}
		return;
	}

	// Bearer mode - check for token in localStorage
	if (mode === 'bearer') {
		const token = localStorage.getItem('token');
		if (!token && window.location.pathname !== '/login') {
			window.location.href = '/login';
		} else if (token) {
			validateToken(token);
		}
	}
}

// Export getAuthMode for components that need to know the auth mode
export { getAuthMode };
