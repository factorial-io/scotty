import { describe, expect, it } from 'vitest';
import { operationTitle } from './operationTitle';

// Every operation name the server uses (docker/*.rs run_operation calls).
describe('operationTitle', () => {
	it.each([
		['create', 'Creating app shop'],
		['run', 'Starting app shop'],
		['stop', 'Stopping app shop'],
		['ttl-stop', 'Stopping app shop (time to live reached)'],
		['rebuild', 'Rebuilding app shop'],
		['purge', 'Purging app shop'],
		['destroy', 'Destroying app shop'],
		['ttl-destroy', 'Destroying app shop (time to live reached)'],
		['action:drush:uli', 'Running custom action drush:uli on shop']
	])('words %s', (operation, title) => {
		expect(operationTitle(operation, 'shop')).toBe(title);
	});

	it('returns null for unknown or missing names', () => {
		expect(operationTitle('migrate', 'shop')).toBeNull();
		expect(operationTitle('ttl-migrate', 'shop')).toBeNull();
		expect(operationTitle(null, 'shop')).toBeNull();
	});

	it('leaves out a missing app', () => {
		expect(operationTitle('stop', null)).toBe('Stopping app');
		expect(operationTitle('action:cr', null)).toBe('Running custom action cr');
	});
});
