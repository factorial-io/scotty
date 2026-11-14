import { describe, expect, it, vi } from 'vitest';

vi.mock('./appsStore', async () => ({
	apps: (await import('svelte/store')).writable([]),
	loadApps: vi.fn(),
	updateAppInfo: vi.fn().mockResolvedValue({})
}));
vi.mock('./tasksStore', async () => ({
	tasks: (await import('svelte/store')).writable({}),
	updateTask: vi.fn(),
	requestAllTasks: vi.fn(),
	requestTaskDetails: vi.fn().mockResolvedValue({})
}));

import { apps, loadApps, updateAppInfo } from './appsStore';
import { requestTaskDetails, tasks } from './tasksStore';
import { handleMessage } from './webSocketStore';

function receive(message: object) {
	handleMessage(new MessageEvent('message', { data: JSON.stringify(message) }));
}

describe('webSocketStore', () => {
	it('refetches the app on AppInfoUpdated', () => {
		receive({ type: 'AppInfoUpdated', data: 'shop' });
		expect(updateAppInfo).toHaveBeenCalledWith('shop');
	});

	it('swallows a failed refetch (the store already removed a 404 app)', async () => {
		vi.mocked(updateAppInfo).mockRejectedValueOnce(new Error('not found'));
		receive({ type: 'AppInfoUpdated', data: 'gone' });
		await Promise.resolve();
		expect(updateAppInfo).toHaveBeenCalledWith('gone');
	});

	it('reloads the list on AppListUpdated', () => {
		receive({ type: 'AppListUpdated' });
		expect(loadApps).toHaveBeenCalled();
	});

	it('resyncs busy apps and running tasks once authenticated', () => {
		apps.set([
			{ name: 'busy', operation: { kind: 'Run' } },
			{ name: 'idle', operation: null }
		] as never);
		tasks.set({ r: { id: 'r', state: 'Running' }, f: { id: 'f', state: 'Finished' } } as never);
		vi.mocked(updateAppInfo).mockClear();

		receive({ type: 'AuthenticationSuccess' });

		expect(updateAppInfo).toHaveBeenCalledTimes(1);
		expect(updateAppInfo).toHaveBeenCalledWith('busy');
		expect(requestTaskDetails).toHaveBeenCalledTimes(1);
		expect(requestTaskDetails).toHaveBeenCalledWith('r');
	});
});
