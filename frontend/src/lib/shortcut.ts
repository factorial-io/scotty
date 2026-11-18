/** `/` opens the app search, except while typing in a field. */
export function isSearchShortcut(e: KeyboardEvent): boolean {
	if (e.key !== '/' || e.metaKey || e.ctrlKey || e.altKey) return false;
	const t = e.target as HTMLElement | null;
	return !t?.closest?.('input, textarea, select, [contenteditable=""], [contenteditable="true"]');
}
