import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';

vi.mock('#lib', () => ({ authenticatedApiCall: vi.fn().mockResolvedValue({}) }));
vi.mock('$app/navigation', () => ({ goto: vi.fn() }));
vi.mock('../../../stores/confirmStore', () => ({ askConfirm: vi.fn() }));
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
import { authenticatedApiCall } from '#lib';
import { askConfirm } from '../../../stores/confirmStore';
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
	}
	const button = (name: string) =>
		[...document.body.querySelectorAll<HTMLButtonElement>('button')].find(
			(b) => b.textContent?.trim() === name
		);

	it('shows the header facts as status and tags', () => {
		render({
			...app(),
			settings: {
				app_blueprint: 'nginx-lagoon',
				time_to_live: { Days: 7 },
				destroy_on_ttl: true,
				basic_auth: null,
				environment: null,
				middlewares: [],
				registry: null
			}
		} as unknown as App);
		const tags = [...document.querySelectorAll('.tag')].map((t) => t.textContent?.trim());
		expect(tags).toEqual([
			'Lifetime: 7 days, then destroyed (not running)',
			'Blueprint: nginx-lagoon'
		]);
		expect(document.body.textContent).toContain('7 days, then destroyed');
		expect(document.querySelector('.chips .st .w')?.textContent).toBe('Running');
	});

	it('counts down to the end of the lifetime of a running app', () => {
		const in3Days = new Date(Date.now() + 3 * 86400_000 + 3600_000).toISOString();
		render({
			...app(),
			settings: {
				time_to_live: { Days: 7 },
				destroy_on_ttl: false,
				basic_auth: null,
				environment: null,
				middlewares: [],
				registry: null
			},
			ttl_expires_at: in3Days
		} as unknown as App);
		const tags = [...document.querySelectorAll('.tag')].map((t) => t.textContent?.trim());
		expect(tags[0]).toBe('Lifetime: 7 days, then stopped in 3 d');
	});

	it('names the running operation, links its task and disables the actions', () => {
		render(app({ operation: 'run', task_id: 't1', started_at: '2020-01-01T00:00:00Z' }));
		expect(document.body.textContent).toMatch(/Starting app \S+, started .*ago/);
		const link = document.body.querySelector<HTMLAnchorElement>('a[href$="/tasks/t1"]');
		expect(link?.textContent).toBe('Follow the task output');
		for (const name of ['Stop', 'Rebuild', 'Purge']) expect(button(name)?.disabled).toBe(true);
	});

	it('enables the actions while idle', () => {
		render(app());
		expect(document.querySelector('.banner')).toBeNull();
		for (const name of ['Stop', 'Rebuild', 'Purge']) expect(button(name)?.disabled).toBe(false);
	});

	it('explains an unmanaged app and offers no actions', () => {
		render({ ...app(), status: 'Unsupported' } as App);
		expect(document.querySelector('.banner b')?.textContent).toBe(
			'Scotty does not manage this app'
		);
		expect(document.body.textContent).toContain('scottyctl app:create');
		expect(document.querySelector('.acts')).toBeNull();
	});

	it('destroys only after the confirmation', async () => {
		const settings = {
			basic_auth: null,
			environment: null,
			middlewares: [],
			time_to_live: 'Forever'
		};
		render({ ...app(), settings } as unknown as App);
		const destroyCalls = () =>
			vi
				.mocked(authenticatedApiCall)
				.mock.calls.filter(([url]) => url === 'apps/destroy/shop');
		const choose = async () => {
			document
				.querySelector('[aria-label="More actions"]')!
				.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
			await vi.waitFor(() => {
				flushSync();
				expect(document.querySelector('[role="menuitem"]')).not.toBeNull();
			});
			(document.querySelector('[role="menuitem"]') as HTMLElement).click();
			await new Promise((r) => setTimeout(r));
		};

		vi.mocked(askConfirm).mockResolvedValueOnce(false);
		await choose();
		expect(askConfirm).toHaveBeenCalledTimes(1);
		expect(destroyCalls()).toHaveLength(0);

		vi.mocked(askConfirm).mockResolvedValueOnce(true);
		await choose();
		expect(destroyCalls()).toHaveLength(1);
	});
});
