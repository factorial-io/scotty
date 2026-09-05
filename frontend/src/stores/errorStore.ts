import { writable } from 'svelte/store';

/** Message shown by the global error dialog; null when closed. */
export const errorMessage = writable<string | null>(null);

export function showError(context: string, err: unknown): void {
	errorMessage.set(`${context}: ${err instanceof Error ? err.message : String(err)}`);
}
