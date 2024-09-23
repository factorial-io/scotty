// place files you want to import through the `$lib` alias in this folder.

export async function apiCall(url: string, options: RequestInit = {}): Promise<unknown> {
	if (typeof window !== 'undefined') {
		const currentToken = localStorage.getItem('token');

		if (currentToken) {
			options.headers = {
				...options.headers,
				Authorization: `Bearer ${currentToken}`
			};
		}
		const response = await fetch(`/api/v1/${url}`, options);
		const result = await response.json();
		return result;
	}
}

export async function validateToken(token: string) {
	const response = await fetch('/api/v1/validate-token', {
		method: 'POST',
		headers: {
			Authorization: `Bearer ${token}`
		}
	});

	if (!response.ok && window.location.pathname !== '/login') {
		// If the token is invalid, redirect to login page
		window.location.href = '/login';
	}
}

export async function checkIfLoggedIn() {
	const token = localStorage.getItem('token');

	if (!token && window.location.pathname !== '/login') {
		// If token is not found, redirect to the login page
		window.location.href = '/login';
	} else {
		// Optionally, you can add further validation of the token by making an API call
		validateToken(token);
	}
}
