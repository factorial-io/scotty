import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';

vi.mock('#lib', () => ({ authenticatedApiCall: vi.fn() }));
vi.mock('$app/navigation', () => ({ goto: vi.fn() }));
vi.mock('$app/state', () => ({ page: { url: new URL('http://localhost/dashboard/create') } }));
vi.mock('../../../stores/confirmStore', () => ({ askConfirm: vi.fn() }));
vi.mock('../../../stores/appsStore', async () => {
	const { writable } = await import('svelte/store');
	return { apps: writable([]), loadApps: vi.fn().mockResolvedValue(undefined) };
});
vi.mock('../../../stores/permissionStore', async () => {
	const { writable } = await import('svelte/store');
	return { permissions: writable([]) };
});
vi.mock('../../../stores/userStore', async () => {
	const { writable } = await import('svelte/store');
	return { authMode: writable('bearer') };
});

import Page from './+page.svelte';
import { authenticatedApiCall } from '#lib';
import { goto } from '$app/navigation';
import { page } from '$app/state';
import { askConfirm } from '../../../stores/confirmStore';
import { apps } from '../../../stores/appsStore';
import { permissions as permissionsStore } from '../../../stores/permissionStore';
import type { Writable } from 'svelte/store';
import type { App, Blueprint, CreateAppRequest } from '../../../types';

const api = vi.mocked(authenticatedApiCall);
const permissions = permissionsStore as unknown as Writable<
	{ name: string; description: string; permissions: string[] }[]
>;

const blueprints: Record<string, Blueprint> = {
	'static-html': {
		name: 'static-html',
		description: 'Serves static HTML',
		actions: {},
		templated: true,
		template: {
			parameters: null,
			images: ['nginx:1'],
			payload: ['html/**'],
			payload_required: ['html/index.html']
		}
	},
	atelier: {
		name: 'atelier',
		description: 'An Atelier instance',
		actions: {},
		templated: true,
		template: {
			parameters: {
				type: 'object',
				properties: {
					SITE_NAME: { type: 'string', title: 'Site name' },
					WORKERS: { type: 'integer' },
					DEBUG: { type: 'boolean' },
					HASH_SALT: { type: 'string', readOnly: true },
					TRUSTED_HOSTS: { type: 'string', readOnly: true }
				},
				required: ['SITE_NAME']
			},
			images: ['atelier:1', 'atelier:2'],
			payload: []
		}
	},
	'nginx-lagoon': {
		name: 'nginx-lagoon',
		description: 'Validation only',
		actions: {},
		templated: false
	}
};

let component: ReturnType<typeof mount> | null = null;
let lastRequest: CreateAppRequest | null = null;

async function settle() {
	for (let i = 0; i < 10; i++) await new Promise((r) => setTimeout(r, 0));
	flushSync();
}

async function render(blueprint?: string) {
	(page.url as URL).search = blueprint ? `?blueprint=${blueprint}` : '';
	component = mount(Page, { target: document.body });
	await settle();
}

const $ = <T extends Element = HTMLInputElement>(sel: string) => document.querySelector<T>(sel)!;
function type(sel: string, value: string) {
	const input = $(sel);
	input.value = value;
	input.dispatchEvent(new Event('input'));
	flushSync();
}
async function pickFolder(...paths: string[]) {
	const files = paths.map((p) => {
		const f = new File(['x'], p.slice(p.lastIndexOf('/') + 1));
		Object.defineProperty(f, 'webkitRelativePath', { value: p });
		return f;
	});
	const folder = $('input[webkitdirectory]');
	Object.defineProperty(folder, 'files', { value: files, configurable: true });
	folder.dispatchEvent(new Event('change', { bubbles: true }));
	await settle();
}
async function submit() {
	$<HTMLFormElement>('form').requestSubmit();
	await settle();
}

describe('create app page', () => {
	beforeEach(() => {
		lastRequest = null;
		api.mockImplementation(async (url, options) => {
			if (url === 'blueprints') return { blueprints };
			lastRequest = JSON.parse(options!.body as string);
			return { task: { id: 'task-1' } };
		});
		permissions.set([{ name: 'default', description: '', permissions: ['view', 'create'] }]);
		apps.set([]);
	});
	afterEach(() => {
		if (component) unmount(component);
		component = null;
		document.body.innerHTML = '';
		vi.clearAllMocks();
	});

	it('offers only templated blueprints, with descriptions', async () => {
		await render();
		const offered = [...document.querySelectorAll('input[name="blueprint"]')].map(
			(i) => (i as HTMLInputElement).value
		);
		expect(offered).toEqual(['static-html', 'atelier']);
		expect(document.body.textContent).toContain('Serves static HTML');
	});

	it('preselects the blueprint from the query', async () => {
		await render('atelier');
		expect($('input[value="atelier"]').checked).toBe(true);
	});

	it('does not submit while a required parameter is empty', async () => {
		await render('atelier');
		type('#app-name', 'demo');
		await submit();
		expect($('#param-SITE_NAME').required).toBe(true);
		expect($('#param-SITE_NAME').validity.valueMissing).toBe(true);
		expect(lastRequest).toBeNull();
	});

	it('hides read-only parameters, sends typed values and the chosen image', async () => {
		await render('atelier');
		expect(document.querySelector('#param-HASH_SALT')).toBeNull();
		expect(document.querySelector('#param-TRUSTED_HOSTS')).toBeNull();
		expect(document.querySelector('input[type="file"]')).toBeNull();

		type('#app-name', 'demo');
		type('#param-SITE_NAME', 'Demo');
		type('#param-WORKERS', '4');
		const img = $<HTMLSelectElement>('#image');
		expect(img.value).toBe('atelier:1');
		img.value = 'atelier:2';
		img.dispatchEvent(new Event('change', { bubbles: true }));
		await submit();

		expect(lastRequest!.blueprint_parameters).toEqual({
			SITE_NAME: 'Demo',
			WORKERS: 4,
			DEBUG: false
		});
		expect(lastRequest!.image).toBe('atelier:2');
		expect(lastRequest!.files.files).toEqual([]);
		expect(goto).toHaveBeenCalledWith('/tasks/task-1');
	});

	it('presets a single scope and sends defaults, basic auth and no image for one image', async () => {
		await render('static-html');
		expect($<HTMLSelectElement>('#scope').value).toBe('default');
		await pickFolder('my-site/index.html');
		type('#app-name', 'site');
		type('#auth-user', 'alice');
		type('input[aria-label="Basic auth password"]', 'secret');
		await submit();

		expect(lastRequest).toMatchObject({
			app_name: 'site',
			requested_scopes: ['default'],
			custom_domains: [],
			settings: {
				app_blueprint: 'static-html',
				scopes: ['default'],
				time_to_live: { Days: 7 },
				basic_auth: ['alice', 'secret'],
				disallow_robots: true,
				registry: null
			}
		});
		expect(lastRequest).not.toHaveProperty('image');
	});

	it('offers only scopes granting create or create:<blueprint>', async () => {
		permissions.set([
			{ name: 'a', description: '', permissions: ['view'] },
			{ name: 'b', description: '', permissions: ['create:static-html'] },
			{ name: 'c', description: '', permissions: ['create:atelier'] }
		]);
		await render('static-html');
		const options = [...$<HTMLSelectElement>('#scope').options].map((o) => o.value);
		expect(options).toEqual(['b']);
	});

	it('names a rejected .env.local and blocks submit', async () => {
		await render('static-html');
		const file = new File(['X=1'], '.env.local');
		Object.defineProperty(file, 'webkitRelativePath', { value: 'my-site/.env.local' });
		const folder = $('input[webkitdirectory]');
		Object.defineProperty(folder, 'files', { value: [file], configurable: true });
		folder.dispatchEvent(new Event('change', { bubbles: true }));
		await settle();

		expect(document.body.textContent).toContain('html/.env.local');
		expect($<HTMLButtonElement>('button[type="submit"]').disabled).toBe(true);
	});

	it('confirms before updating an existing app', async () => {
		apps.set([{ name: 'report-x' } as App]);
		vi.mocked(askConfirm).mockResolvedValue(false);
		await render('static-html');
		await pickFolder('my-site/index.html');
		type('#app-name', 'Report X');
		expect(document.body.textContent).toContain('Created as report-x');
		await submit();
		expect(askConfirm).toHaveBeenCalledWith(
			expect.objectContaining({ title: 'Update report-x?' })
		);
		expect(lastRequest).toBeNull();

		vi.mocked(askConfirm).mockResolvedValue(true);
		await submit();
		expect(lastRequest!.app_name).toBe('Report X');
	});

	it('confirms when the name cannot be normalised exactly, even without a match', async () => {
		vi.mocked(askConfirm).mockResolvedValue(false);
		await render('static-html');
		await pickFolder('my-site/index.html');
		type('#app-name', 'Große Seite');
		await submit();
		expect(askConfirm).toHaveBeenCalledWith(
			expect.objectContaining({ body: expect.stringContaining('Große Seite') })
		);
		expect(lastRequest).toBeNull();
	});

	it('presets scope, TTL and basic auth from a matching app', async () => {
		permissions.set([
			{ name: 'default', description: '', permissions: ['create'] },
			{ name: 'client-a', description: '', permissions: ['create'] }
		]);
		apps.set([
			{
				name: 'report-x',
				settings: {
					scopes: ['client-a'],
					time_to_live: 'Forever',
					basic_auth: ['u', 'p']
				}
			} as unknown as App
		]);
		vi.mocked(askConfirm).mockResolvedValue(true);
		await render('static-html');
		await pickFolder('my-site/index.html');
		type('#app-name', 'report-x');
		expect($<HTMLSelectElement>('#scope').value).toBe('client-a');
		expect($('#ttl').value).toBe('forever');
		expect($('#auth-user').value).toBe('u');
		expect($('input[aria-label="Basic auth password"]').value).toBe('p');
		await submit();
		expect(lastRequest!.settings).toMatchObject({
			scopes: ['client-a'],
			time_to_live: 'Forever',
			basic_auth: ['u', 'p']
		});
	});

	it('keeps the values and shows the server message when create fails', async () => {
		api.mockImplementation(async (url) => {
			if (url === 'blueprints') return { blueprints };
			throw new Error('SITE_NAME is too long');
		});
		await render('atelier');
		type('#app-name', 'demo');
		type('#param-SITE_NAME', 'Demo');
		await submit();
		expect(document.body.textContent).toContain('SITE_NAME is too long');
		expect($('#param-SITE_NAME').value).toBe('Demo');
		expect(goto).not.toHaveBeenCalled();
	});

	it('requires the upload: lists the required path and blocks submit', async () => {
		await render('static-html');
		type('#app-name', 'site');
		expect(document.body.textContent).toContain('html/index.html');
		expect($<HTMLButtonElement>('button[type="submit"]').disabled).toBe(true);
		await submit();
		expect(lastRequest).toBeNull();
	});

	it('flags a folder without index and accepts one with index', async () => {
		await render('static-html');
		type('#app-name', 'site');
		await pickFolder('my-site/about.html');
		expect(document.body.textContent).toContain('Required but missing');
		expect($<HTMLButtonElement>('button[type="submit"]').disabled).toBe(true);

		await pickFolder('my-site/index.html');
		expect(document.body.textContent).not.toContain('Required but missing');
		expect($<HTMLButtonElement>('button[type="submit"]').disabled).toBe(false);
	});
});
