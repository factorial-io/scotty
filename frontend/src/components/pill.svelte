<script lang="ts">
	export let text = '';
	export let color = 'bg-gray-300 dark:bg-gray-700';
	export let textColor = 'text-white dark:text-black';
	export let additionalClasses = '';
</script>

<span class="{textColor} px-3 py-1 rounded-full text-xs {color} {additionalClasses}">
	{#if text}
		{text}
	{:else}
		<slot />
	{/if}
</span>
