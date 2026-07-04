/**
 * Landing-page resume state.
 *
 * When an unauthenticated user clicks "Start App" on the landing page, the
 * intent is stored in sessionStorage so that whichever login flow follows
 * (OAuth callback or password login) can send the user back to the landing
 * page to start the app, instead of dropping them on the dashboard.
 */

const PENDING_START_KEY = 'scotty_landing_pending_start';

export interface PendingLandingStart {
	appName: string;
	returnUrl: string | null;
}

function storageAvailable(): boolean {
	return typeof sessionStorage !== 'undefined';
}

/** Remember that the user wants to start this app after logging in. */
export function storePendingLandingStart(pending: PendingLandingStart): void {
	if (!storageAvailable()) return;
	sessionStorage.setItem(PENDING_START_KEY, JSON.stringify(pending));
}

/**
 * Read and consume the pending landing start. Returns the landing page path
 * (with `autostart=true` so the app starts without another click), or null
 * when no start is pending. The stored entry is always removed so a stale
 * intent cannot resurface on a later login.
 */
export function consumePendingLandingRedirect(): string | null {
	if (!storageAvailable()) return null;
	const raw = sessionStorage.getItem(PENDING_START_KEY);
	if (!raw) return null;
	sessionStorage.removeItem(PENDING_START_KEY);
	try {
		const pending = JSON.parse(raw) as PendingLandingStart;
		if (!pending.appName) return null;
		const params = new URLSearchParams({ autostart: 'true' });
		if (pending.returnUrl) {
			params.set('return_url', pending.returnUrl);
		}
		return `/landing/${encodeURIComponent(pending.appName)}?${params}`;
	} catch {
		return null;
	}
}
