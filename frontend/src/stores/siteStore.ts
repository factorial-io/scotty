import { writable } from 'svelte/store';

/** Server domain and version from `/api/v1/info`, loaded once by the layout. */
export const siteInfo = writable<{
	domain: string;
	version: string;
	/** Set when the server enables SuisseIntl (`api.suisse_fonts_url`). */
	suisse_fonts_url?: string | null;
}>({ domain: '…', version: '0.0.0' });
