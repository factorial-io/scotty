import { afterEach, describe, expect, it } from 'vitest';
import { applySuisseFonts, suisseFontFaces } from './fonts';

describe('SuisseIntl fonts', () => {
	afterEach(() => applySuisseFonts(null));

	it('points the faces at the configured folder', () => {
		const css = suisseFontFaces('https://config.factorial.io/fonts');
		expect(css).toContain(
			'url("https://config.factorial.io/fonts/SuisseIntl-Regular-Web/S/SuisseIntl-Regular-WebS.woff2")'
		);
		expect(css).toContain('font-family: SuisseIntl; font-weight: 600');
		expect(css).toContain('SuisseIntlMono-Regular-Web/S/SuisseIntlMono-Regular-WebS.woff2');
	});

	it('adds no rules without the setting', () => {
		applySuisseFonts(null);
		expect(document.getElementById('scotty-suisse-fonts')).toBeNull();
		applySuisseFonts('https://fonts.example');
		expect(document.querySelectorAll('#scotty-suisse-fonts')).toHaveLength(1);
		applySuisseFonts(undefined);
		expect(document.getElementById('scotty-suisse-fonts')).toBeNull();
	});
});
