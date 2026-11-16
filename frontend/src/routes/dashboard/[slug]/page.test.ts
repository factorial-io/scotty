import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';

vi.mock('#lib', () => ({ authenticatedApiCall: vi.fn().mockResolvedValue({}) }));
vi.mock('$app/navigation', () => ({ goto: vi.fn() }));
vi.mock('../../../stores/permissionStore', async () => {
	const { readable } = await import('svelte/store');
	return {
		getAppPermissions: () => ({ view: true, manage: true, destroy: true }),
		permissionsLoaded: readable(true),
		permissionsLoading: readable(false),
		loadUserPermissions: vi.fn()
	};
});

import Page from './+page.svelte';
import type { App } from '../../../types';

function app(operation: App['operation'] = null): App {
	return {
		name: 'shop',
		status: 'Running',
		services: [],
		settings: null,
		operation
	} as unknown as App;
}

describe('app detail page', () => {
	let component: ReturnType<typeof mount> | null = null;

	afterEach(() => {
		if (component) unmount(component);
		component = null;
		document.body.innerHTML = '';
	});

	function render(data: App) {
		component = mount(Page, { target: document.body, props: { data } });
		flushSync();
		return [...document.body.querySelectorAll<HTMLButtonElement>('.join button')];
	}

	it('names the running operation, links its task and disables the actions', () => {
		const buttons = render(
			app({ operation: 'run', task_id: 't1', started_at: '2020-01-01T00:00:00Z' })
		);
		const link = document.body.querySelector<HTMLAnchorElement>('a[href$="/tasks/t1"]');
		expect(link?.textContent).toMatch(/run running since .*ago/);
		expect(buttons.map((b) => b.textContent?.trim())).toEqual([
			'Run',
			'Stop',
			'Purge',
			'Rebuild'
		]);
		expect(buttons.every((b) => b.disabled)).toBe(true);
	});

	it('enables the actions and shows no pill while idle', () => {
		const buttons = render(app());
		expect(document.body.textContent).not.toMatch(/running since/);
		expect(buttons.every((b) => !b.disabled)).toBe(true);
	});
});
