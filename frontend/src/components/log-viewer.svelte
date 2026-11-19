<script lang="ts">
	import Icon from '@iconify/svelte';
	import { tick } from 'svelte';
	import type { OutputLine } from '../types';
	import {
		copyText,
		filterByLevel,
		findMatches,
		formatTime,
		guessLevel,
		levelCounts,
		splitForHighlight,
		type LevelFilter
	} from '../lib/logView';
	import { showToast } from '../stores/toastStore';
	import { showError } from '../stores/errorStore';

	let {
		lines,
		live,
		stopped = false,
		range = 500,
		onRangeChange
	}: {
		lines: OutputLine[];
		/** The server follows this stream (follow: true on LogsStreamStarted). */
		live: boolean;
		stopped?: boolean;
		/** Lines requested: 500, 2000 or null for everything since the container started. */
		range?: number | null;
		onRangeChange?: (range: number | null) => void;
	} = $props();

	const RANGES: [string, string][] = [
		['500', 'Last 500 lines'],
		['2000', 'Last 2,000 lines'],
		['all', 'Since container start']
	];
	const LEVELS: [LevelFilter, string][] = [
		['all', 'All'],
		['warn', 'Warnings'],
		['error', 'Errors']
	];

	let query = $state('');
	let level = $state<LevelFilter>('all');
	let showTs = $state(true);
	let wrap = $state(true);
	let expanded = $state(false);
	let follow = $state(true);
	let paused = $state(false);
	let fresh = $state(0);
	let current = $state(-1);

	let out: HTMLDivElement | undefined = $state();
	let search: HTMLInputElement | undefined = $state();

	let visible = $derived(filterByLevel(lines, level));
	let counts = $derived(levelCounts(lines));
	let matches = $derived(findMatches(visible, query));
	let cur = $derived(current >= 0 ? matches[current] : undefined);
	let following = $derived(live && follow && !paused);

	const fmt = (n: number) => n.toLocaleString('en');
	let status = $derived(
		`${fmt(visible.length)} of ${fmt(lines.length)} lines` +
			(stopped
				? ' · container stopped'
				: !live
					? ''
					: paused
						? ' · paused while you scroll'
						: follow
							? ' · following'
							: '')
	);

	// Count lines that arrive while the user reads further up.
	let lastSeq = -1;
	$effect(() => {
		const last = lines.length ? Number(lines[lines.length - 1].sequence) : -1;
		if (paused && lastSeq >= 0 && last > lastSeq) fresh += last - lastSeq;
		lastSeq = last;
	});

	// Stick to the newest line while following.
	$effect(() => {
		void visible.length;
		if (!following || !out) return;
		tick().then(() => out && (out.scrollTop = out.scrollHeight));
	});

	// A new search starts at its first match.
	$effect(() => {
		void query;
		current = query && matches.length ? 0 : -1;
	});

	$effect(() => {
		if (current < 0 || !out) return;
		void cur;
		tick().then(() => out?.querySelector('mark.cur')?.scrollIntoView({ block: 'center' }));
	});

	function goMatch(dir: 1 | -1) {
		if (!matches.length) return;
		current = (current + dir + matches.length) % matches.length;
		paused = live;
	}

	function jumpToLatest() {
		paused = false;
		fresh = 0;
		if (out) out.scrollTop = out.scrollHeight;
	}

	function onScroll() {
		if (!out) return;
		const atBottom = out.scrollHeight - out.scrollTop - out.clientHeight < 24;
		if (atBottom && paused) {
			paused = false;
			fresh = 0;
		} else if (!atBottom && !paused && live && follow) {
			paused = true;
		}
	}

	function onKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape' && expanded) {
			expanded = false;
		} else if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'f') {
			e.preventDefault();
			search?.focus();
		}
	}

	function onSearchKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter') {
			e.preventDefault();
			goMatch(e.shiftKey ? -1 : 1);
		}
	}

	async function copy() {
		const n = visible.length;
		try {
			await navigator.clipboard.writeText(copyText(visible, showTs));
			showToast(`Copied ${fmt(n)} ${n === 1 ? 'line' : 'lines'}`);
		} catch (err) {
			showError('Copying is blocked by the browser', err);
		}
	}

	function onRange(e: Event) {
		const v = (e.currentTarget as HTMLSelectElement).value;
		paused = false;
		fresh = 0;
		onRangeChange?.(v === 'all' ? null : Number(v));
	}

	const isCurrent = (i: number, start: number) => cur?.line === i && cur?.start === start;
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div class="logv" class:expanded role="region" aria-label="Logs" onkeydown={onKeydown}>
	<div class="logv-tools">
		<label class="field">
			<Icon icon="ph:magnifying-glass" class="ic sm" aria-hidden="true" />
			<input
				bind:this={search}
				bind:value={query}
				type="search"
				placeholder="Search logs"
				aria-label="Search logs"
				onkeydown={onSearchKeydown}
			/>
			{#if query}
				<span class="mono" aria-live="polite"
					>{matches.length ? `${current + 1}/${matches.length}` : 'No match'}</span
				>
			{/if}
			<button
				class="ibtn"
				type="button"
				aria-label="Previous match"
				disabled={!matches.length}
				onclick={() => goMatch(-1)}><Icon icon="ph:caret-up" class="ic sm" /></button
			>
			<button
				class="ibtn"
				type="button"
				aria-label="Next match"
				disabled={!matches.length}
				onclick={() => goMatch(1)}><Icon icon="ph:caret-down" class="ic sm" /></button
			>
		</label>
		<div
			class="pills"
			role="group"
			aria-label="Filter by level"
			title="Levels are guessed from the loaded lines"
		>
			{#each LEVELS as [key, label] (key)}
				<button
					class="pill"
					type="button"
					aria-pressed={level === key}
					onclick={() => (level = key)}
				>
					{#if key === 'error'}<span class="st err"><i class="sw"></i></span>{/if}
					{#if key === 'warn'}<span class="st warn"><i class="sw"></i></span>{/if}
					{label}<span class="mono">{fmt(counts[key])}</span>
				</button>
			{/each}
		</div>
		<select
			aria-label="How many lines"
			value={range === null ? 'all' : String(range)}
			onchange={onRange}
		>
			{#each RANGES as [value, label] (value)}
				<option {value}>{label}</option>
			{/each}
		</select>
		<span class="grow"></span>
		<div class="tg">
			<button
				class="ibtn"
				type="button"
				aria-pressed={showTs}
				title="Show timestamps"
				aria-label="Show timestamps"
				onclick={() => (showTs = !showTs)}><Icon icon="ph:clock" class="ic" /></button
			>
			<button
				class="ibtn"
				type="button"
				aria-pressed={wrap}
				title="Wrap long lines"
				aria-label="Wrap long lines"
				onclick={() => (wrap = !wrap)}><Icon icon="ph:text-align-left" class="ic" /></button
			>
			<button
				class="ibtn"
				type="button"
				title="Copy visible lines"
				aria-label="Copy visible lines"
				onclick={copy}><Icon icon="ph:copy" class="ic" /></button
			>
			<button
				class="ibtn"
				type="button"
				title={expanded ? 'Leave full screen (Esc)' : 'Expand (Esc to leave)'}
				aria-label={expanded ? 'Leave full screen' : 'Expand'}
				aria-pressed={expanded}
				onclick={() => (expanded = !expanded)}
				><Icon
					icon={expanded ? 'ph:arrows-in-simple' : 'ph:arrows-out-simple'}
					class="ic"
				/></button
			>
		</div>
	</div>

	<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
	<div
		class="out"
		class:nots={!showTs}
		class:nowrap={!wrap}
		bind:this={out}
		onscroll={onScroll}
		tabindex="0"
		aria-label="Log lines"
	>
		{#each visible as line, i (String(line.sequence))}
			{@const lvl = guessLevel(line.content)}
			<div class="ln" class:e={lvl === 'error'} class:w={lvl === 'warn'}>
				<span class="n">{Number(line.sequence) + 1}</span>
				<span class="ts">{formatTime(line.timestamp)}</span>
				<span
					>{#each splitForHighlight(line.content, query) as seg (seg.start)}{#if seg.match}<mark
								class:cur={isCurrent(i, seg.start)}>{seg.text}</mark
							>{:else}{seg.text}{/if}{/each}</span
				>
			</div>
		{/each}
	</div>

	<div class="logv-foot">
		<span>{status}</span>
		<span class="meta">Levels are guessed from the loaded lines</span>
		<label class="switch">
			<input type="checkbox" bind:checked={follow} disabled={!live} />Follow new lines
		</label>
		{#if paused && fresh > 0}
			<button class="jump" type="button" onclick={jumpToLatest}>
				<Icon icon="ph:arrow-down" class="ic sm" aria-hidden="true" />
				{fresh} new {fresh === 1 ? 'line' : 'lines'} · Jump to latest
			</button>
		{/if}
	</div>
</div>
