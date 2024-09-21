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
