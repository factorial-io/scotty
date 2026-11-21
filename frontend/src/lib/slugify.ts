/**
 * The server's app-name rule (scotty-core/src/utils/slugify.rs): ASCII-lowercase,
 * anything else becomes `-`, at most two dashes in a row, no leading dash, one
 * trailing dash dropped.
 * ponytail: NFKD + dropping combining marks stands in for deunicode, so accents
 * fold (`é` → `e`) but scripts without a decomposition (`こ`, `ß`) become dashes.
 */
export function slugify(s: string): string {
	let slug = '';
	let dashes = 1;
	for (const c of s.normalize('NFKD').replace(/\p{M}/gu, '')) {
		if (/^[a-zA-Z0-9]$/.test(c)) {
			slug += c.toLowerCase();
			dashes = 0;
		} else if (dashes < 2) {
			slug += '-';
			dashes++;
		}
	}
	return slug.endsWith('-') ? slug.slice(0, -1) : slug;
}

/** False when non-ASCII characters survive folding, so the server may normalise differently. */
export const slugifyIsExact = (s: string): boolean =>
	!/\P{ASCII}/u.test(s.normalize('NFKD').replace(/\p{M}/gu, ''));
