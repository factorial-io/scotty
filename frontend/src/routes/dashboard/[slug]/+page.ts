import { error } from '@sveltejs/kit';
import { updateAppInfo } from '../../../stores/appsStore';

export const ssr = false;
export const prerender = false;

export async function load({ params }) {
	const result = await updateAppInfo(params.slug);
	if ('error' in result && result.error === true) {
		error(404, result.message);
	}
	return result;
}
