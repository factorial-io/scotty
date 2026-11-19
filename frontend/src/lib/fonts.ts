/**
 * `@font-face` rules for SuisseIntl, loaded from `api.suisse_fonts_url`
 * (laid out like https://config.factorial.io/fonts). Without the setting no
 * rules are added and the bundled Inter and JetBrains Mono apply.
 */
const FACES: [family: string, weight: number, file: string][] = [
	['SuisseIntl', 400, 'SuisseIntl-Regular'],
	['SuisseIntl', 600, 'SuisseIntl-SemiBold'],
	['SuisseIntlMono', 400, 'SuisseIntlMono-Regular']
];

export function suisseFontFaces(baseUrl: string): string {
	return FACES.map(
		([family, weight, file]) =>
			`@font-face { font-family: ${family}; font-weight: ${weight}; font-style: normal; font-display: swap; src: url("${baseUrl}/${file}-Web/S/${file}-WebS.woff2") format("woff2"); }`
	).join('\n');
}

/** Add the rules once; a later call with another URL replaces them. */
export function applySuisseFonts(baseUrl: string | null | undefined): void {
	const id = 'scotty-suisse-fonts';
	document.getElementById(id)?.remove();
	if (!baseUrl) return;
	const style = document.createElement('style');
	style.id = id;
	style.textContent = suisseFontFaces(baseUrl);
	document.head.append(style);
}
