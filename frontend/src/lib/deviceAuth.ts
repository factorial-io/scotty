/**
 * Logic for the `/device` page: a logged-in user approves or denies a
 * `scottyctl auth:login` device code (RFC 8628 user interaction step).
 */

import { ApiError, authenticatedApiCall } from '$lib';

export interface DeviceRequest {
	user_code: string;
	client_name: string;
	scope: string;
}

export type DeviceState =
	| { kind: 'idle' }
	| { kind: 'loading' }
	| { kind: 'pending'; request: DeviceRequest }
	| { kind: 'approved' }
	| { kind: 'denied' }
	| { kind: 'invalid' }
	| { kind: 'error'; message: string };

/** Normalize what the user typed or what came in `?code=`: trimmed, upper case, no spaces. */
export function normalizeUserCode(code: string | null | undefined): string {
	return (code ?? '').replace(/\s+/g, '').toUpperCase();
}

/** The prefilled user code from the page URL (`/device?code=XXXX-XXXX`). */
export function codeFromUrl(url: URL): string {
	return normalizeUserCode(url.searchParams.get('code'));
}

/** Scope string as a list for display. */
export function scopeList(scope: string): string[] {
	return scope.split(/\s+/).filter(Boolean);
}

function failure(error: unknown): DeviceState {
	// 404: unknown, expired or already decided.
	if (error instanceof ApiError && error.status === 404) return { kind: 'invalid' };
	return { kind: 'error', message: error instanceof Error ? error.message : String(error) };
}

/** Look up a pending device authorization by user code. */
export async function lookupDeviceCode(code: string): Promise<DeviceState> {
	const userCode = normalizeUserCode(code);
	if (!userCode) return { kind: 'idle' };
	try {
		const request = (await authenticatedApiCall(
			`oauth/device/${encodeURIComponent(userCode)}`
		)) as DeviceRequest;
		return { kind: 'pending', request };
	} catch (error) {
		return failure(error);
	}
}

/** Approve or deny the device authorization. */
export async function decideDeviceCode(code: string, approve: boolean): Promise<DeviceState> {
	try {
		await authenticatedApiCall(`oauth/device/${approve ? 'approve' : 'deny'}`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ user_code: normalizeUserCode(code) })
		});
		return { kind: approve ? 'approved' : 'denied' };
	} catch (error) {
		return failure(error);
	}
}
