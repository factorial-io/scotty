import { afterEach, describe, expect, it } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import FormatBasicAuth from './format-basic-auth.svelte';

describe('format-basic-auth', () => {
	let component: ReturnType<typeof mount> | null = null;
	afterEach(() => {
		if (component) unmount(component);
		document.body.innerHTML = '';
	});

	it('masks the password, reveals and hides it again', () => {
		component = mount(FormatBasicAuth, {
			target: document.body,
			props: { auth: ['review', 's3cret'] }
		});
		flushSync();
		const text = () => document.querySelector('.mono')!.textContent;
		const button = document.querySelector('button')!;
		expect(text()).toBe('review / ••••••');

		button.click();
		flushSync();
		expect(text()).toBe('review / s3cret');
		expect(button.getAttribute('aria-pressed')).toBe('true');

		button.click();
		flushSync();
		expect(text()).toBe('review / ••••••');
	});
});
