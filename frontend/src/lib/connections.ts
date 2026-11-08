/**
 * Logic for the `/connections` page: list and revoke the user's MCP and CLI
 * grants; admins can list everyone's.
 */

import { authenticatedApiCall } from '$lib';

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

export async function listConnections(all = false): Promise<Connection[]> {
	return (await authenticatedApiCall(`connections${all ? '?all=true' : ''}`)) as Connection[];
}

/** Revoke a connection and return the list without it. */
export async function revokeConnection(
	connections: Connection[],
	id: string
): Promise<Connection[]> {
	await authenticatedApiCall(`connections/${encodeURIComponent(id)}`, { method: 'DELETE' });
	return connections.filter((c) => c.id !== id);
}
