export const ssr = false;
export const prerender = false;

export async function load({ params, url }: { params: { slug: string }; url: URL }) {
	return {
		appName: params.slug,
		returnUrl: url.searchParams.get('return_url') || null,
		autostart: url.searchParams.get('autostart') === 'true'
	};
}
