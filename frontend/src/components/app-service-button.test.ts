import { afterEach, describe, expect, it } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import AppServiceButton from './app-service-button.svelte';
import type { AppService } from '../types';

const service = (status: string, domains: string[]) =>
	({ service: 'nginx', status, domains, use_tls: true }) as unknown as AppService;

describe('app-service-button', () => {
	let component: ReturnType<typeof mount> | null = null;
	const render = (s: AppService) => {
		component = mount(AppServiceButton, { target: document.body, props: { service: s } });
		flushSync();
		return [...document.body.querySelectorAll('a')];
	};
	afterEach(() => {
		if (component) unmount(component);
		document.body.innerHTML = '';
	});

	it('links a running service to its domain', () => {
		const [a] = render(service('Running', ['shop.example.dev']));
		expect(a.getAttribute('href')).toBe('https://shop.example.dev');
		expect(a.textContent).toContain('nginx');
		expect(a.classList.contains('muted')).toBe(false);
	});

	it('mutes a stopped service but still links its domain', () => {
		const [a] = render(service('Exited', ['shop.example.dev']));
		expect(a.getAttribute('href')).toBe('https://shop.example.dev');
		expect(a.classList.contains('muted')).toBe(true);
	});

	it('renders nothing for a service without a domain', () => {
		expect(render(service('Running', []))).toHaveLength(0);
	});
});
