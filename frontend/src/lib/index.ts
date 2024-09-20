// place files you want to import through the `$lib` alias in this folder.

export async function apiCall(url: string, options: RequestInit = {}): Promise<Response> {
	if (typeof window !== 'undefined') {
		const currentToken = localStorage.getItem('token');

		if (currentToken) {
			options.headers = {
				...options.headers,
				Authorization: `Bearer ${currentToken}`
			};
		}

		return fetch(`/api/v1/${url}`, options);
	}
}
