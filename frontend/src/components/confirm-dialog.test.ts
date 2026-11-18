import { afterEach, beforeAll, beforeEach, describe, expect, it } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import ConfirmDialog from './confirm-dialog.svelte';
import { askConfirm } from '../stores/confirmStore';

// jsdom lacks showModal/close; emulate them.
beforeAll(() => {
	HTMLDialogElement.prototype.showModal = function () {
		this.setAttribute('open', '');
	};
	HTMLDialogElement.prototype.close = function () {
		if (!this.open) return;
		this.removeAttribute('open');
		this.dispatchEvent(new Event('close'));
	};
});

const request = {
	title: 'Destroy shop?',
	body: "Scotty stops all containers, removes them and deletes the app's volumes.",
	consequence: 'This cannot be undone. 4 services and their data are deleted with it.',
	confirmLabel: 'Destroy app'
};

describe('confirm-dialog', () => {
	let component: ReturnType<typeof mount> | null = null;
	beforeEach(() => {
		component = mount(ConfirmDialog, { target: document.body });
	});
	afterEach(() => {
		if (component) unmount(component);
		component = null;
		document.body.innerHTML = '';
	});

	const dialog = () => document.body.querySelector('dialog')!;
	const button = (label: string) =>
		[...document.body.querySelectorAll('button')].find((b) => b.textContent?.trim() === label)!;

	it('names object and consequence and focuses the safe choice', () => {
		void askConfirm(request);
		flushSync();
		expect(dialog().open).toBe(true);
		expect(dialog().textContent).toContain('Destroy shop?');
		expect(dialog().textContent).toContain('4 services and their data');
		expect(document.activeElement).toBe(button('Keep it'));
	});

	it('resolves false when closed without confirming (Escape)', async () => {
		const answer = askConfirm(request);
		flushSync();
		dialog().close(); // what Escape does
		expect(await answer).toBe(false);
	});

	it('resolves true only on the destructive button', async () => {
		const answer = askConfirm(request);
		flushSync();
		button('Destroy app').click();
		expect(await answer).toBe(true);
		flushSync();
		expect(dialog().open).toBe(false);
	});
});
