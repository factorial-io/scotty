import { writable } from 'svelte/store';

export type ThemeChoice = 'light' | 'dark' | 'system';

// Keep in sync with the inline script in src/app.html, which applies the theme
// before the first paint.
export const THEME_KEY = 'scotty-theme';

function readChoice(): ThemeChoice {
	try {
		const v = localStorage.getItem(THEME_KEY);
		return v === 'light' || v === 'dark' ? v : 'system';
	} catch {
		return 'system';
	}
}

function systemIsDark(): boolean {
	return typeof matchMedia === 'function' && matchMedia('(prefers-color-scheme: dark)').matches;
}

/** Set data-theme (tokens.css) and .is-dark (components.css) on <html>. */
export function applyTheme(choice: ThemeChoice): void {
	const dark = choice === 'dark' || (choice === 'system' && systemIsDark());
	const root = document.documentElement;
	root.setAttribute('data-theme', dark ? 'dark' : 'light');
	root.classList.toggle('is-dark', dark);
}

function createThemeStore() {
	const { subscribe, set } = writable<ThemeChoice>('system');
	let media: MediaQueryList | null = null;
	const onSystemChange = () => applyTheme(readChoice());

	return {
		subscribe,
		/** Read the stored choice, apply it and follow the OS while the choice is System. */
		init() {
			const choice = readChoice();
			set(choice);
			applyTheme(choice);
			if (typeof matchMedia === 'function' && !media) {
				media = matchMedia('(prefers-color-scheme: dark)');
				media.addEventListener('change', onSystemChange);
			}
		},
		choose(choice: ThemeChoice) {
			try {
				if (choice === 'system') localStorage.removeItem(THEME_KEY);
				else localStorage.setItem(THEME_KEY, choice);
			} catch {
				// Storage blocked (private mode, policy): the choice holds for this page only.
			}
			set(choice);
			applyTheme(choice);
		}
	};
}

export const theme = createThemeStore();
