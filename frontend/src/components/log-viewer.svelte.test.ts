import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, tick, unmount } from 'svelte';
import { get } from 'svelte/store';
import LogViewer from './log-viewer.svelte';
import { errorMessage } from '../stores/errorStore';
import { toasts } from '../stores/toastStore';
import { isSearchShortcut } from '../lib/shortcut';
import type { OutputLine } from '../types';

beforeAll(() => {
	// jsdom has no layout: stub what the viewer calls.
	Element.prototype.scrollIntoView = vi.fn();
});

let seq = 0;
const mk = (content: string): OutputLine =>
	({
		content,
		stream: 'Stdout',
		timestamp: '2026-11-19T10:00:00.000Z',
		sequence: seq++
	}) as unknown as OutputLine;

let component: ReturnType<typeof mount> | null = null;
afterEach(() => {
	if (component) unmount(component);
	component = null;
	document.body.innerHTML = '';
	errorMessage.set(null);
});

function render(props: { lines: OutputLine[]; live: boolean; stopped?: boolean }) {
	component = mount(LogViewer, { target: document.body, props });
	flushSync();
}

const q = <T extends Element>(sel: string) => document.body.querySelector<T>(sel)!;
const button = (label: string) =>
	[...document.body.querySelectorAll('button')].find((b) =>
		b.textContent?.replace(/\s+/g, ' ').trim().startsWith(label)
	)!;
const search = () => q<HTMLInputElement>('input[type=search]');

function type(text: string) {
	search().value = text;
	search().dispatchEvent(new Event('input', { bubbles: true }));
	flushSync();
}

describe('log-viewer search', () => {
	it('counts matches and moves with Enter', async () => {
		render({
			lines: [mk('a timed out'), mk('ok'), mk('timed out twice: timed out')],
			live: false
		});
		type('timed out');
		await tick();
		expect(document.body.querySelectorAll('mark')).toHaveLength(3);
		expect(document.body.textContent).toContain('1/3');
		search().dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
		flushSync();
		expect(document.body.textContent).toContain('2/3');
		expect(q('mark.cur').textContent).toBe('timed out');
	});

	it('says when nothing matches', () => {
		render({ lines: [mk('abc')], live: false });
		type('zzz');
		expect(document.body.textContent).toContain('No match');
		expect(document.body.querySelectorAll('mark')).toHaveLength(0);
	});

	it('keeps a typed slash in the search field', () => {
		render({ lines: [], live: false });
		const e = new KeyboardEvent('keydown', { key: '/', bubbles: true });
		Object.defineProperty(e, 'target', { value: search() });
		expect(isSearchShortcut(e)).toBe(false);
	});

	it('takes over Ctrl+F only inside the viewer', () => {
		render({ lines: [mk('x')], live: false });
		q<HTMLElement>('.out').dispatchEvent(
			new KeyboardEvent('keydown', { key: 'f', ctrlKey: true, bubbles: true })
		);
		expect(document.activeElement).toBe(search());
	});
});

describe('log-viewer filter and copy', () => {
	const lines = () => [
		mk('error: one'),
		mk('fatal two'),
		mk('GET / 200'),
		mk('panic three'),
		mk('[warn] slow'),
		mk('ERR four')
	];

	it('shows only errors with their count', () => {
		render({ lines: lines(), live: false });
		expect(button('Errors').textContent).toContain('4');
		button('Errors').click();
		flushSync();
		expect(document.body.querySelectorAll('.ln')).toHaveLength(4);
		expect(document.body.textContent).toContain('4 of 6 lines');
	});

	it('copies the visible lines', async () => {
		const writeText = vi.fn().mockResolvedValue(undefined);
		Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true });
		render({ lines: lines(), live: false });
		button('Errors').click();
		flushSync();
		q<HTMLButtonElement>('button[aria-label="Copy visible lines"]').click();
		await Promise.resolve();
		await Promise.resolve();
		expect(writeText).toHaveBeenCalledWith(
			'10:00:00.000 error: one\n10:00:00.000 fatal two\n10:00:00.000 panic three\n10:00:00.000 ERR four'
		);
		expect(get(toasts).at(-1)?.message).toBe('Copied 4 lines');
	});

	it('reports a refused clipboard in the error dialog', async () => {
		const writeText = vi.fn().mockRejectedValue(new Error('denied'));
		Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true });
		render({ lines: lines(), live: false });
		q<HTMLButtonElement>('button[aria-label="Copy visible lines"]').click();
		await Promise.resolve();
		await Promise.resolve();
		expect(get(errorMessage)).toContain('Copying is blocked');
	});
});

describe('log-viewer follow, pause and full screen', () => {
	it('counts new lines while paused and jumps back', async () => {
		const props = $state({ lines: [mk('one'), mk('two')], live: true });
		render(props);
		const out = q<HTMLElement>('.out');
		Object.defineProperty(out, 'scrollHeight', { value: 1000, configurable: true });
		Object.defineProperty(out, 'clientHeight', { value: 200, configurable: true });
		out.scrollTop = 0;
		out.dispatchEvent(new Event('scroll'));
		flushSync();
		expect(document.body.textContent).toContain('paused while you scroll');

		props.lines = [...props.lines, ...Array.from({ length: 12 }, (_, i) => mk(`new ${i}`))];
		flushSync();
		expect(button('12 new lines')).toBeTruthy();
		expect(button('12 new lines').textContent).toContain('Jump to latest');

		button('12 new lines').click();
		flushSync();
		expect(document.body.textContent).toContain('following');
		expect(document.body.textContent).not.toContain('Jump to latest');
	});

	it('disables follow for a stopped container', () => {
		render({ lines: [mk('bye')], live: false, stopped: true });
		expect(q<HTMLInputElement>('.switch input').disabled).toBe(true);
		expect(document.body.textContent).toContain('1 of 1 lines · container stopped');
	});

	it('leaves full screen on Escape', () => {
		render({ lines: [mk('x')], live: false });
		q<HTMLButtonElement>('button[aria-label="Expand"]').click();
		flushSync();
		expect(q('.logv').classList.contains('expanded')).toBe(true);
		q<HTMLElement>('.out').dispatchEvent(
			new KeyboardEvent('keydown', { key: 'Escape', bubbles: true })
		);
		flushSync();
		expect(q('.logv').classList.contains('expanded')).toBe(false);
	});
});
