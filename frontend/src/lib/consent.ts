/**
 * Logic for the `/oauth/consent/[id]` page: a logged-in user approves or
 * denies an MCP client's authorization request, optionally dropping scopes.
 */

import { ApiError, authenticatedApiCall } from '$lib';

export interface ConsentRequest {
	id: string;
	client_id: string;
	client_name: string;
	redirect_host: string;
	loopback_only: boolean;
	scopes: string[];
}

export type ConsentState =
	| { kind: 'loading' }
	| { kind: 'pending'; request: ConsentRequest }
	| { kind: 'redirect'; url: string }
	| { kind: 'invalid' }
	| { kind: 'error'; message: string };

/** Plain descriptions for the known scopes; unknown scopes show their name. */
export const SCOPE_DESCRIPTIONS: Record<string, string> = {
	read: 'View apps, blueprints and tasks',
	create: 'Create apps',
	manage: 'Stop, start and rebuild apps',
	destroy: 'Destroy apps',
	offline_access: 'Stay connected without asking again'
};

export function describeScope(scope: string): string {
	return SCOPE_DESCRIPTIONS[scope] ?? scope;
}

/** The ticked scopes, in the order the client requested them. */
export function selectedScopes(requested: string[], ticked: Record<string, boolean>): string[] {
	return requested.filter((scope) => ticked[scope]);
}

export function canApprove(requested: string[], ticked: Record<string, boolean>): boolean {
	return selectedScopes(requested, ticked).length > 0;
}

function failure(error: unknown): ConsentState {
	// 404: unknown, expired, already decided or claimed by someone else.
	if (error instanceof ApiError && error.status === 404) return { kind: 'invalid' };
	return { kind: 'error', message: error instanceof Error ? error.message : String(error) };
}

const base = (id: string) => `oauth/consent/${encodeURIComponent(id)}`;

export async function loadConsent(id: string): Promise<ConsentState> {
	try {
		return {
			kind: 'pending',
			request: (await authenticatedApiCall(base(id))) as ConsentRequest
		};
	} catch (error) {
		return failure(error);
	}
}

/**
 * Approve with the given scopes (or deny when `scopes` is null). On success
 * returns the client redirect URL the browser must navigate to (another origin).
 */
export async function decideConsent(id: string, scopes: string[] | null): Promise<ConsentState> {
	try {
		const options: RequestInit =
			scopes === null
				? { method: 'POST' }
				: {
						method: 'POST',
						headers: { 'Content-Type': 'application/json' },
						body: JSON.stringify({ scopes })
					};
		const { redirect } = (await authenticatedApiCall(
			`${base(id)}/${scopes === null ? 'deny' : 'approve'}`,
			options
		)) as { redirect: string };
		return { kind: 'redirect', url: redirect };
	} catch (error) {
		return failure(error);
	}
}
