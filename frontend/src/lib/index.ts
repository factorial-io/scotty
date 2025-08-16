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
		const response = await fetch('/api/v1/login', {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ password: '' }), // Empty password to get auth info
			credentials: 'include'
		});

		if (response.ok) {
			const result = await response.json();
			authMode = result.auth_mode || 'bearer';
		} else {
			authMode = 'bearer'; // fallback
		}
	} catch (error) {
		console.warn('Failed to detect auth mode, defaulting to bearer:', error);
		authMode = 'bearer';
	}

	return authMode;
}

export async function apiCall(url: string, options: RequestInit = {}): Promise<unknown> {
	if (typeof window !== 'undefined') {
		const mode = await getAuthMode();

		// Always include cookies for OAuth mode
		options.credentials = 'include';

		// Add bearer token only in bearer mode
		if (mode === 'bearer') {
			const currentToken = localStorage.getItem('token');
			if (currentToken) {
				options.headers = {
					...options.headers,
					Authorization: `Bearer ${currentToken}`
				};
			}
		}

		const response = await fetch(`/api/v1/${url}`, options);

		// Handle 401 Unauthorized based on auth mode
		if (response.status === 401) {
			handleUnauthorized(mode);
			return Promise.reject(new Error('Unauthorized'));
		}

		const result = await response.json();
		return result;
	}
}

function handleUnauthorized(mode: AuthMode) {
	if (window.location.pathname === '/login') {
		return; // Already on login page
	}

	switch (mode) {
		case 'oauth':
			// In OAuth mode, redirect to oauth2-proxy login
			window.location.href = '/oauth2/start';
			break;
		case 'bearer':
			// In bearer mode, redirect to token login page
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
	const response = await fetch('/api/v1/validate-token', {
		method: 'POST',
		headers: {
			Authorization: `Bearer ${token}`
		},
		credentials: 'include'
	});

	if (!response.ok && window.location.pathname !== '/login') {
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

	// For OAuth mode, cookies handle authentication automatically
	// Just make a simple API call to verify we're authenticated
	if (mode === 'oauth') {
		try {
			await apiCall('info');
		} catch (error) {
			// apiCall will handle 401 and redirect appropriately
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
