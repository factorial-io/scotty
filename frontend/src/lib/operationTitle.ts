const VERBS: Record<string, string> = {
	create: 'Creating',
	run: 'Starting',
	stop: 'Stopping',
	rebuild: 'Rebuilding',
	purge: 'Purging',
	destroy: 'Destroying'
};

/**
 * Words an app operation name (`rebuild`, `ttl-stop`, `action:drush:uli`)
 * as a sentence, e.g. "Stopping app shop (time to live reached)".
 * Returns null for a missing or unknown name; callers fall back to the
 * task's command or the raw name.
 */
export function operationTitle(
	operation: string | null | undefined,
	app: string | null | undefined
): string | null {
	if (!operation) return null;
	const target = app ? ` ${app}` : '';
	if (operation.startsWith('action:')) {
		const name = operation.slice('action:'.length);
		return `Running custom action ${name}${app ? ` on ${app}` : ''}`;
	}
	const ttl = operation.startsWith('ttl-');
	const verb = VERBS[ttl ? operation.slice('ttl-'.length) : operation];
	if (!verb) return null;
	return `${verb} app${target}${ttl ? ' (time to live reached)' : ''}`;
}
