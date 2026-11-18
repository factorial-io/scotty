import { writable } from 'svelte/store';

export interface ConfirmRequest {
	/** e.g. "Destroy shop?" */
	title: string;
	/** What happens, e.g. "Scotty stops all containers, removes them and deletes the app's volumes." */
	body: string;
	/** The consequence that cannot be taken back, shown highlighted. Optional. */
	consequence?: string;
	/** Label of the destructive button, e.g. "Destroy app". */
	confirmLabel: string;
	/** Label of the safe button. */
	cancelLabel?: string;
}

interface Pending extends ConfirmRequest {
	resolve: (ok: boolean) => void;
}

export const pendingConfirm = writable<Pending | null>(null);

/** Ask in the confirmation dialog; resolves true only on the destructive button. */
export function askConfirm(request: ConfirmRequest): Promise<boolean> {
	return new Promise((resolve) => {
		pendingConfirm.update((current) => {
			current?.resolve(false); // a newer question replaces an open one
			return { ...request, resolve };
		});
	});
}

export function answerConfirm(ok: boolean): void {
	pendingConfirm.update((current) => {
		current?.resolve(ok);
		return null;
	});
}
