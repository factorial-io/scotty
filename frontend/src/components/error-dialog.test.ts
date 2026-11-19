import { afterEach, beforeAll, describe, expect, it } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { get } from 'svelte/store';
import ErrorDialog from './error-dialog.svelte';
import { errorMessage, showError } from '../stores/errorStore';

// jsdom has <dialog> but not showModal/close; emulate the parts the component uses.
beforeAll(() => {
	HTMLDialogElement.prototype.showModal = function () {
		this.setAttribute('open', '');
	};
	HTMLDialogElement.prototype.close = function () {
		this.removeAttribute('open');
		this.dispatchEvent(new Event('close'));
	};
});

describe('error-dialog', () => {
	let component: ReturnType<typeof mount> | null = null;
	afterEach(() => {
		if (component) unmount(component);
		component = null;
		errorMessage.set(null);
		document.body.innerHTML = '';
	});

	it('opens with the message and clears the store on close', () => {
		component = mount(ErrorDialog, { target: document.body });
		showError('Run shop', new Error('lacks manage permission'));
		flushSync();

		const dialog = document.body.querySelector('dialog')!;
		expect(dialog.open).toBe(true);
		expect(dialog.textContent).toContain('Run shop: lacks manage permission');

		dialog.close(); // Escape and the Close buttons all end in close()
		expect(get(errorMessage)).toBeNull();
	});
});
