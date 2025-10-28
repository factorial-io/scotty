import { error } from '@sveltejs/kit';
import { updateAppInfo } from '../../../../stores/appsStore';
import type { App, AppService } from '../../../../types';

export const ssr = false;
export const prerender = false;

export async function load({ params }: { params: { slug: string; service: string } }) {
	const result = await updateAppInfo(params.slug);
	if ('error' in result && result.error === true) {
		error(404, result.message);
	}

	const app = result as App;
	const service = app.services.find((s: AppService) => s.service === params.service);

	if (!service) {
		error(404, `Service '${params.service}' not found in app '${params.slug}'`);
	}

	return {
		app,
		service
	};
}
