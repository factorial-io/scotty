import { writable } from 'svelte/store';

/** Confirmations and information only. Failures go to the error dialog (errorStore). */
export interface Toast {
	id: number;
	message: string;
}

export const TOAST_DURATION_MS = 5000;

const { subscribe, update } = writable<Toast[]>([]);
const timers = new Map<
	number,
	{ timer: ReturnType<typeof setTimeout>; left: number; since: number }
>();
let nextId = 1;

function schedule(id: number, ms: number) {
	timers.set(id, { timer: setTimeout(() => dismissToast(id), ms), left: ms, since: Date.now() });
}

export const toasts = { subscribe };

export function showToast(message: string): number {
	const id = nextId++;
	update((list) => [...list, { id, message }]);
	schedule(id, TOAST_DURATION_MS);
	return id;
}

export function dismissToast(id: number): void {
	const t = timers.get(id);
	if (t) clearTimeout(t.timer);
	timers.delete(id);
	update((list) => list.filter((x) => x.id !== id));
}

/** Stop the countdown while the pointer rests on the toast. */
export function pauseToast(id: number): void {
	const t = timers.get(id);
	if (!t) return;
	clearTimeout(t.timer);
	t.left = Math.max(0, t.left - (Date.now() - t.since));
}

/** Resume the countdown; give the reader at least 1.5 s after the pointer leaves. */
export function resumeToast(id: number): void {
	const t = timers.get(id);
	if (!t) return;
	schedule(id, Math.max(1500, t.left));
}
