/**
 * Logic for the `/connections` page: list and revoke the user's own MCP and
 * CLI grants. (The server's admin view, `?all=true`, has no UI.)
 */

import { writable } from 'svelte/store';
import { authenticatedApiCall } from '#lib';

/** The user's own connection count for the sidebar; null until loaded (design D6). */
export const connectionCount = writable<number | null>(null);

export interface Connection {
	id: string;
	kind: 'mcp' | 'cli';
	client_id: string;
	client_name: string;
	redirect_host: string;
	scope: string;
	subject: string;
	created_at: string;
	last_used_at: string | null;
	expires_at: string;
}

export async function listConnections(): Promise<Connection[]> {
	const list = (await authenticatedApiCall('connections')) as Connection[];
	connectionCount.set(list.length);
	return list;
}

/** Load the count once after sign-in; a failure leaves the count hidden. */
export async function refreshConnectionCount(): Promise<void> {
	try {
		await listConnections();
	} catch {
		connectionCount.set(null);
	}
}

/** Revoke a connection and return the list without it. */
export async function revokeConnection(
	connections: Connection[],
	id: string
): Promise<Connection[]> {
	await authenticatedApiCall(`connections/${encodeURIComponent(id)}`, { method: 'DELETE' });
	connectionCount.update((n) => (n === null ? n : Math.max(0, n - 1)));
	return connections.filter((c) => c.id !== id);
}
