<script>
	export let heading = 'stdOut';
	export let output = '';
</script>

{#if output}
	<h3 class="my-4 text-lg font-bold">
		{heading}
	</h3>
	<div class="mockup-code my-4 text-xs bg-black text-white">
		{#each output.split('\n') as line, index (line + index)}
			<pre data-prefix={index + 1}><code>{line}</code></pre>
		{/each}
	</div>
{/if}
