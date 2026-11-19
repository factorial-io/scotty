import { describe, expect, it } from 'vitest';
import { isSearchShortcut } from './shortcut';

function press(target: HTMLElement, key = '/') {
	let hit = false;
	target.addEventListener('keydown', (e) => (hit = isSearchShortcut(e)), { once: true });
	target.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true }));
	return hit;
}

describe('isSearchShortcut', () => {
	it('opens the search from the page', () => {
		expect(press(document.body)).toBe(true);
		expect(press(document.body, 'a')).toBe(false);
	});

	it('is ignored in text fields', () => {
		const input = document.body.appendChild(document.createElement('input'));
		const area = document.body.appendChild(document.createElement('textarea'));
		expect(press(input)).toBe(false);
		expect(press(area)).toBe(false);
	});
});
