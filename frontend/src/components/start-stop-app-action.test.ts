import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';

vi.mock('../stores/appsStore', () => ({ runApp: vi.fn(), stopApp: vi.fn() }));
vi.mock('../stores/permissionStore', async () => {
	const { readable } = await import('svelte/store');
	return { hasPermission: () => true, permissionsLoaded: readable(true) };
});

import StartStopAppAction from './start-stop-app-action.svelte';
import { runApp } from '../stores/appsStore';
import { tasks } from '../stores/tasksStore';
import type { App } from '../types';

function app(operation: App['operation'] = null): App {
	return { name: 'shop', status: 'Stopped', services: [], operation } as unknown as App;
}

const rebuild = { operation: 'rebuild', task_id: 't9', started_at: '2026-01-01T00:00:00Z' };

describe('start-stop-app-action', () => {
	let component: ReturnType<typeof mount> | null = null;

	beforeEach(() => {
		tasks.set({});
		vi.mocked(runApp).mockReset();
	});

	afterEach(() => {
		if (component) unmount(component);
		component = null;
		document.body.innerHTML = '';
	});

	function render(props: { app: App }) {
		component = mount(StartStopAppAction, { target: document.body, props });
		flushSync();
		return document.body.querySelector('button')!;
	}

	it('shows the busy indicator and is disabled while an operation runs', () => {
		const button = render({ app: app(rebuild) });
		expect(button.disabled).toBe(true);
		expect(button.querySelector('.loading-spinner')).not.toBeNull();
	});

	it('is enabled and idle without an operation', () => {
		const button = render({ app: app() });
		expect(button.disabled).toBe(false);
		expect(button.querySelector('.loading-spinner')).toBeNull();
	});

	it('shows the failure of the task it started from the tasks store', async () => {
		vi.mocked(runApp).mockResolvedValue('t1');
		const button = render({ app: app() });
		button.click();
		await vi.waitFor(() => expect(runApp).toHaveBeenCalledWith('shop'));
		await Promise.resolve();

		tasks.set({ t1: { id: 't1', state: 'Failed', command: 'run' } as never });
		flushSync();

		expect(document.body.querySelector('.tooltip-error')).not.toBeNull();
	});
});
