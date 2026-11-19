import { error } from '@sveltejs/kit';
import { ApiError } from '#lib';
import { requestTaskDetails } from '../../../stores/tasksStore';

export const ssr = false;
export const prerender = false;

export async function load({ params }) {
	let result;
	try {
		result = await requestTaskDetails(params.slug);
	} catch (err) {
		// Tasks live in server memory: after a restart an old link is a 404, not a crash.
		if (err instanceof ApiError) error(err.status, err.message);
		throw err;
	}
	if ('error' in result && result.error === true) {
		error(404, result.message);
	}
	return result;
}
