import { describe, expect, it, vi } from 'vitest';
import { ApiError } from '#lib';

vi.mock('../../../stores/tasksStore', () => ({
	requestTaskDetails: vi.fn(async () => {
		throw new ApiError('Task not found', 404);
	})
}));

import { load } from './+page.js';

describe('task detail loader', () => {
	it('turns an unknown task into a 404, not a crash', async () => {
		await expect(load({ params: { slug: 'gone' } } as never)).rejects.toMatchObject({
			status: 404
		});
	});
});
