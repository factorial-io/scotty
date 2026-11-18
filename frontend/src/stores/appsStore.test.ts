import { beforeEach, describe, expect, it, vi } from 'vitest';
import { get } from 'svelte/store';

vi.mock('#lib', async () => {
	class ApiError extends Error {
		constructor(
			message: string,
			public status: number
		) {
			super(message);
		}
	}
	return { ApiError, authenticatedApiCall: vi.fn() };
});
const { connected } = vi.hoisted(() => ({ connected: { value: true } }));
vi.mock('./webSocketStore', async () => {
	const { readable } = await import('svelte/store');
	return {
		isConnected: {
			subscribe: (fn: (v: boolean) => void) => readable(connected.value).subscribe(fn)
		}
	};
});
vi.mock('./tasksStore', async () => {
	const { writable } = await import('svelte/store');
	return { monitorTask: vi.fn(), tasks: writable({}) };
});
vi.mock('./errorStore', () => ({ showError: vi.fn() }));

import { ApiError, authenticatedApiCall } from '#lib';
import { monitorTask, tasks } from './tasksStore';
import { dismissToast, toasts } from './toastStore';
import { apps, runApp, updateAppInfo } from './appsStore';
import type { App } from '../types';

function app(name: string, operation: App['operation'] = null): App {
	return { name, status: 'Stopped', services: [], operation } as unknown as App;
}

const running = { operation: 'run', task_id: 't1', started_at: '2026-01-01T00:00:00Z' };

describe('appsStore', () => {
	beforeEach(() => {
		apps.set([app('shop'), app('blog')]);
		connected.value = true;
		vi.mocked(authenticatedApiCall).mockReset();
		vi.mocked(monitorTask).mockReset();
		get(toasts).forEach((t) => dismissToast(t.id));
		tasks.set({});
	});

	it('stores the app data returned by an action, so the app is busy at once', async () => {
		vi.mocked(authenticatedApiCall).mockResolvedValue({
			task: { id: 't1' },
			app_data: app('shop', running)
		});

		expect(await runApp('shop')).toBe('t1');
		expect(get(apps).find((a) => a.name === 'shop')?.operation).toEqual(running);
		expect(monitorTask).not.toHaveBeenCalled();
	});

	it('polls the task without an authenticated WebSocket', async () => {
		connected.value = false;
		vi.mocked(authenticatedApiCall).mockResolvedValue({ task: { id: 't1' } });

		await runApp('shop');

		expect(monitorTask).toHaveBeenCalledWith('t1', expect.any(Function));
	});

	it('refetches the app when the action is refused as busy', async () => {
		vi.mocked(authenticatedApiCall)
			.mockRejectedValueOnce(new ApiError('rebuild is running', 409))
			.mockResolvedValueOnce(app('shop', running));

		await expect(runApp('shop')).rejects.toThrow('rebuild is running');
		await vi.waitFor(() =>
			expect(get(apps).find((a) => a.name === 'shop')?.operation).toEqual(running)
		);
	});

	it('updateAppInfo replaces or inserts the app', async () => {
		vi.mocked(authenticatedApiCall)
			.mockResolvedValueOnce(app('shop', running))
			.mockResolvedValueOnce(app('new'));

		await updateAppInfo('shop');
		await updateAppInfo('new');

		expect(get(apps).map((a) => a.name)).toEqual(['shop', 'blog', 'new']);
		expect(get(apps)[0].operation).toEqual(running);
	});

	it('updateAppInfo removes the app on 404', async () => {
		vi.mocked(authenticatedApiCall).mockRejectedValue(new ApiError('not found', 404));

		await expect(updateAppInfo('shop')).rejects.toThrow();

		expect(get(apps).map((a) => a.name)).toEqual(['blog']);
	});

	it('toasts an accepted action and its finish, nothing on failure', async () => {
		vi.mocked(authenticatedApiCall).mockResolvedValueOnce({
			task: { id: 't-toast' },
			app_data: app('shop', running)
		});
		await runApp('shop');
		expect(get(toasts).map((t) => t.message)).toEqual(['Run started for shop']);

		tasks.set({ 't-toast': { id: 't-toast', state: 'Finished' } } as never);
		expect(get(toasts).map((t) => t.message)).toEqual([
			'Run started for shop',
			'Run finished for shop'
		]);
	});

	it('shows no toast when the action is refused with 403', async () => {
		vi.mocked(authenticatedApiCall).mockRejectedValueOnce(new ApiError('Forbidden', 403));
		await expect(runApp('shop')).rejects.toThrow('Forbidden');
		expect(get(toasts)).toEqual([]);
	});
});
