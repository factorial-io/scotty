import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('$lib', () => ({ authenticatedApiCall: vi.fn() }));

import { authenticatedApiCall } from '$lib';
import { listConnections, revokeConnection, type Connection } from './connections';

const api = vi.mocked(authenticatedApiCall);

const conn = (id: string): Connection => ({
	id,
	kind: 'mcp',
	client_id: 'c',
	client_name: 'Claude',
	redirect_host: 'claude.ai',
	scope: 'read create',
	subject: 'user@example.com',
	created_at: '2026-11-01T10:00:00Z',
	last_used_at: null,
	expires_at: '2026-12-01T10:00:00Z'
});

describe('connections page logic', () => {
	beforeEach(() => api.mockReset());

	it('lists own or all connections', async () => {
		api.mockResolvedValue([conn('a')]);
		await listConnections();
		expect(api).toHaveBeenLastCalledWith('connections');
		await listConnections(true);
		expect(api).toHaveBeenLastCalledWith('connections?all=true');
	});

	it('revokes a connection and removes its row', async () => {
		api.mockResolvedValue(null);
		const rows = await revokeConnection([conn('a'), conn('b')], 'a');
		expect(api).toHaveBeenCalledWith('connections/a', { method: 'DELETE' });
		expect(rows.map((c) => c.id)).toEqual(['b']);
	});

	it('keeps the row when revoking fails', async () => {
		api.mockRejectedValueOnce(new Error('boom'));
		const rows = [conn('a')];
		await expect(revokeConnection(rows, 'a')).rejects.toThrow('boom');
		expect(rows).toHaveLength(1);
	});
});
