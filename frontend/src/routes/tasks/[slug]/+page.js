import { error } from '@sveltejs/kit';
import { requestTaskDetails } from '../../../stores/tasksStore';

export const ssr = false;
export const prerender = false;

export async function load({ params }) {
	let result = await requestTaskDetails(params.slug);
	if ('error' in result && result.error === true) {
		error(404, result.message);
	}
	return result;
}
