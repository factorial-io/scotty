<script lang="ts">
	import type { OutputLine, OutputStreamType } from '../types';
	import { onMount } from 'svelte';
	import Icon from '@iconify/svelte';

	export let lines: OutputLine[] = [];
	export let heading: string = 'Output';
	let showTimestamps: boolean = false;
	export let loading: boolean = false;
	export let maxHeight: string = '400px';

	let outputContainer: HTMLElement;
	let shouldAutoScroll = true;

	// Auto-scroll to bottom when new lines are added
	$: if (lines && outputContainer && shouldAutoScroll) {
		scrollToBottom();
	}

	function scrollToBottom() {
		if (outputContainer) {
			outputContainer.scrollTop = outputContainer.scrollHeight;
		}
	}

	function handleScroll() {
		if (outputContainer) {
			const { scrollTop, scrollHeight, clientHeight } = outputContainer;
			// If user is near the bottom (within 50px), keep auto-scrolling
			shouldAutoScroll = scrollTop + clientHeight >= scrollHeight - 50;
		}
	}

	function formatTimestamp(timestamp: string): string {
		const date = new Date(timestamp);
		return date.toLocaleTimeString('en-US', {
			hour12: false,
			hour: '2-digit',
			minute: '2-digit',
			second: '2-digit'
		});
	}

	function getStreamTypeClass(stream: OutputStreamType): string {
		switch (stream) {
			case 'Stderr':
				return 'text-red-400';
			case 'Stdout':
				return 'text-gray-100';
			case 'Status':
				return 'text-blue-400';
			case 'StatusError':
				return 'text-red-500';
			case 'Progress':
				return 'text-yellow-400';
			case 'Info':
				return 'text-cyan-400';
			default:
				return 'text-gray-100';
		}
	}

	function getStreamTypePrefix(stream: OutputStreamType): string {
		switch (stream) {
			case 'Stderr':
				return '[stderr]';
			case 'Stdout':
				return '[stdout]';
			case 'Status':
				return '[status]';
			case 'StatusError':
				return '[error]';
			case 'Progress':
				return '[progress]';
			case 'Info':
				return '[info]';
			default:
				return '';
		}
	}

	function getStreamTypeIcon(stream: OutputStreamType): string | null {
		switch (stream) {
			case 'Stderr':
			case 'Stdout':
				return null; // No icons for basic stdout/stderr
			case 'Status':
				return 'ph:info';
			case 'StatusError':
				return 'ph:x-circle';
			case 'Progress':
				return 'ph:spinner';
			case 'Info':
				return 'ph:lightbulb';
			default:
				return null;
		}
	}

	function toggleTimestamps() {
		showTimestamps = !showTimestamps;
	}

	onMount(() => {
		if (outputContainer) {
			scrollToBottom();
		}
	});
</script>

<!-- Always show the output component -->
<div class="my-4">
		<div class="flex items-center justify-between mb-2">
			<h3 class="text-lg font-bold">{heading}</h3>
			<div class="flex items-center gap-2 text-sm">
				{#if lines.length > 0}
					<span class="text-gray-500">{lines.length} lines</span>
				{/if}
				{#if loading}
					<span class="loading loading-spinner loading-sm"></span>
					<span class="text-gray-500">Loading...</span>
				{/if}
			</div>
		</div>

		<div
			class="mockup-code bg-black text-white text-xs overflow-auto"
			style="max-height: {maxHeight}"
			bind:this={outputContainer}
			on:scroll={handleScroll}
		>
			{#if loading && lines.length === 0}
				<div class="p-4 text-center text-gray-500">
					<span class="loading loading-spinner loading-md"></span>
					<div class="mt-2">Loading output...</div>
				</div>
			{:else}
				{#each lines as line, index (line.sequence)}
					<pre
						data-prefix={index + 1}
						class={getStreamTypeClass(line.stream)}
					><code>{#if showTimestamps}<span class="text-gray-500 mr-2">{formatTimestamp(line.timestamp)}</span>{/if}<span class="inline-flex items-center mr-2">{#if getStreamTypeIcon(line.stream)}<Icon icon={getStreamTypeIcon(line.stream) || ''} class="w-3 h-3 mr-1" />{:else}<span class="w-3 h-3 mr-1 inline-block"></span>{/if}{#if showTimestamps}<span class={getStreamTypeClass(line.stream)}>{getStreamTypePrefix(line.stream)}</span>{/if}</span>{line.content}</code></pre>
				{/each}

				{#if lines.length === 0 && !loading}
					<div class="p-4 text-center text-gray-500">
						No output available
					</div>
				{/if}
			{/if}

		</div>

		{#if lines.length > 0}
			<div class="mt-2 flex items-center gap-2">
				<button
					class="btn btn-sm btn-outline"
					class:btn-disabled={shouldAutoScroll}
					disabled={shouldAutoScroll}
					on:click={scrollToBottom}
				>
					Scroll to bottom ↓
				</button>

				<button
					class="btn btn-sm {showTimestamps ? 'btn-primary' : 'btn-outline'}"
					on:click={toggleTimestamps}
				>
					{showTimestamps ? 'Hide' : 'Show'} Timestamps
				</button>
			</div>
		{/if}
</div>

<style>
	.mockup-code {
		border-radius: 0.5rem;
		font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
		line-height: 1.4;
	}

	.mockup-code pre {
		margin: 0;
		white-space: pre-wrap;
		word-break: break-word;
	}

	.mockup-code pre[data-prefix]:before {
		content: attr(data-prefix);
		display: inline-block;
		text-align: right;
		width: 3rem;
		margin-right: 1rem;
		opacity: 0.5;
		font-size: 0.875em;
		border-right: 1px solid #374151;
		padding-right: 0.5rem;
	}

	/* Scroll behavior */
	.mockup-code {
		scroll-behavior: smooth;
	}

	/* Custom scrollbar for webkit browsers */
	.mockup-code::-webkit-scrollbar {
		width: 8px;
		height: 8px;
	}

	.mockup-code::-webkit-scrollbar-track {
		background: #1f2937;
		border-radius: 4px;
	}

	.mockup-code::-webkit-scrollbar-thumb {
		background: #4b5563;
		border-radius: 4px;
	}

	.mockup-code::-webkit-scrollbar-thumb:hover {
		background: #6b7280;
	}
</style>