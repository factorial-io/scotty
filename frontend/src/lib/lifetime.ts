import type { App, AppTtl } from '../types';
import { formatRelativeTime } from './relative-time';

export function formatTtl(ttl: AppTtl): string {
	if (typeof ttl === 'string') return ttl;
	return ttl.Days ? `${ttl.Days} days` : `${ttl.Hours} hours`;
}

/** What the TTL checker does at the end: mirrors the server's rule. */
export function endOfLife(app: App): 'destroyed' | 'stopped' {
	return app.status !== 'Unsupported' && app.settings?.destroy_on_ttl ? 'destroyed' : 'stopped';
}

/** "Lifetime: 7 days, then destroyed in 3 d", "(not running)", "(due now)" or "Lifetime: Forever". */
export function formatLifetime(app: App, now: Date): string {
	const ttl = app.settings?.time_to_live ?? { Days: 7 };
	if (ttl === 'Forever') return 'Lifetime: Forever';
	const at = app.ttl_expires_at;
	const left = !at
		? '(not running)'
		: new Date(at).getTime() <= now.getTime()
			? '(due now)'
			: formatRelativeTime(now, at);
	return `Lifetime: ${formatTtl(ttl)}, then ${endOfLife(app)} ${left}`;
}
