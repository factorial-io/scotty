import { error } from '@sveltejs/kit';
import { ApiError } from '#lib';
import { updateAppInfo } from '../../../stores/appsStore';

export const ssr = false;
export const prerender = false;

export async function load({ params }: { params: { slug: string } }) {
	let result;
	try {
		result = await updateAppInfo(params.slug);
	} catch (err) {
		// An unknown or removed app is a 404 page, not a crash.
		if (err instanceof ApiError) error(err.status, err.message);
		throw err;
	}
	if ('error' in result && result.error === true) {
		error(404, result.message);
	}
	return result;
}
