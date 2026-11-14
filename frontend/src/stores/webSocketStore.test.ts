import { describe, expect, it, vi } from 'vitest';

vi.mock('./appsStore', () => ({
	loadApps: vi.fn(),
	updateAppInfo: vi.fn().mockResolvedValue({})
}));

import { loadApps, updateAppInfo } from './appsStore';
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
});
