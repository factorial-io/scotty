<script lang="ts">
	import { getGravatarUrl, getUserInitials } from '$lib/gravatar';

	export let email: string = '';
	export let name: string = '';
	export let size: 'xs' | 'sm' | 'md' | 'lg' | 'xl' = 'md';
	export let shape: 'circle' | 'square' = 'circle';

	let imageError = false;

	$: sizeClass = {
		xs: 'w-6 h-6',
		sm: 'w-8 h-8',
		md: 'w-12 h-12',
		lg: 'w-16 h-16',
		xl: 'w-24 h-24'
	}[size];

	$: shapeClass = shape === 'circle' ? 'rounded-full' : 'rounded';

	$: gravatarUrl = getGravatarUrl(email, getSizePixels(size), 'mp'); // 'mp' = mystery person (Gravatar default)
	$: initials = getUserInitials(name, email);

	function getSizePixels(size: string): number {
		const sizeMap = { xs: 24, sm: 32, md: 48, lg: 64, xl: 96 };
		return sizeMap[size as keyof typeof sizeMap] || 48;
	}

	function handleImageLoad() {
		imageError = false;
	}

	function handleImageError() {
		imageError = true;
	}
</script>

<div class="avatar">
	<div
		class="{sizeClass} {shapeClass} {imageError
			? 'bg-primary text-primary-content'
			: 'bg-gray-200'} flex items-center justify-center overflow-hidden"
	>
		{#if !imageError}
			<img
				src={gravatarUrl}
				alt="Avatar for {name || email}"
				class="w-full h-full object-cover"
				on:load={handleImageLoad}
				on:error={handleImageError}
			/>
		{/if}

		{#if imageError || !gravatarUrl}
			<span class="text-sm font-medium">
				{initials}
			</span>
		{/if}
	</div>
</div>
