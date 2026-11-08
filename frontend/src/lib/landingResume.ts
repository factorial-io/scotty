/**
 * Landing-page resume state.
 *
 * When an unauthenticated user clicks "Start App" on the landing page, the
 * intent is stored in sessionStorage so that whichever login flow follows
 * (OAuth callback or password login) can send the user back to the landing
 * page to start the app, instead of dropping them on the dashboard.
 */

import { goto } from '$app/navigation';
import { resolve } from '$app/paths';

const PENDING_START_KEY = 'scotty_landing_pending_start';
const RETURN_PATH_KEY = 'scotty_return_path';

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

/**
 * Remember the page to return to after the next login (e.g. `/device?code=X`
 * or the page that got a 401). Only same-origin paths are kept, so a stored
 * value can never redirect off-site.
 */
export function storeReturnPath(path: string): void {
	if (!storageAvailable() || !isSafeReturnPath(path)) return;
	sessionStorage.setItem(RETURN_PATH_KEY, path);
}

/** Read and remove the stored return path; null when none or unsafe. */
export function consumeReturnPath(): string | null {
	if (!storageAvailable()) return null;
	const path = sessionStorage.getItem(RETURN_PATH_KEY);
	sessionStorage.removeItem(RETURN_PATH_KEY);
	return path && isSafeReturnPath(path) ? path : null;
}

function isSafeReturnPath(path: string): boolean {
	return (
		path.startsWith('/') &&
		!path.startsWith('//') &&
		!path.startsWith('/\\') &&
		path !== '/login' &&
		!path.startsWith('/login?') &&
		!path.startsWith('/oauth/callback')
	);
}

/**
 * Navigate to the pending landing-page app start (if any), else the stored
 * return path, else the dashboard. Call after any successful login.
 */
export async function redirectAfterLogin(): Promise<void> {
	// Consume both so a stale entry cannot resurface on a later login.
	const landing = consumePendingLandingRedirect();
	const returnPath = consumeReturnPath();
	// @ts-expect-error - Type-safe routing doesn't support dynamic redirect paths
	await goto(resolve(landing ?? returnPath ?? '/dashboard'));
}
