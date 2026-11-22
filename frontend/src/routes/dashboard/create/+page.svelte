<script lang="ts">
	import { onMount } from 'svelte';
	import Icon from '@iconify/svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { authenticatedApiCall } from '#lib';
	import { apps, loadApps } from '../../../stores/appsStore';
	import { permissions } from '../../../stores/permissionStore';
	import { authMode } from '../../../stores/userStore';
	import { askConfirm } from '../../../stores/confirmStore';
	import { setTitle } from '../../../stores/titleStore';
	import { slugify, slugifyIsExact } from '../../../lib/slugify';
	import {
		MAX_BODY,
		encodeFiles,
		mapPayloadPaths,
		missingRequired,
		readFolder,
		readZip,
		type MappedPayload,
		type RawEntries
	} from '../../../lib/blueprintPayload';
	import type {
		AppTtl,
		Blueprint,
		BlueprintsResponse,
		CreateAppRequest,
		RunningAppContext
	} from '../../../types';

	type Value = string | number | boolean | null | undefined;

	let blueprints = $state<(Blueprint & { id: string })[]>([]);
	let loadError = $state<string | null>(null);
	let chosen = $state('');
	let params = $state<Record<string, Value>>({});
	let appName = $state('');
	let scopeChoice = $state('');
	let imageChoice = $state('');
	let ttl = $state('7d');
	let authUser = $state('');
	let authPass = $state('');
	let upload = $state<{ label: string; entries: RawEntries; mapped: MappedPayload } | null>(null);
	let uploadError = $state<string | null>(null);
	let submitting = $state(false);
	let submitError = $state<string | null>(null);

	let blueprint = $derived(blueprints.find((b) => b.id === chosen));
	let template = $derived(blueprint?.template);
	let properties = $derived(
		Object.entries(template?.parameters?.properties ?? {}).filter(([, p]) => !p.readOnly)
	);
	let required = $derived(new Set(template?.parameters?.required ?? []));
	let images = $derived(template?.images ?? []);
	let image = $derived(images.includes(imageChoice) ? imageChoice : images[0]);
	let patterns = $derived(template?.payload ?? []);
	let payloadRequired = $derived(template?.payload_required ?? []);
	let missing = $derived(
		missingRequired(upload?.mapped.files.map((f) => f.name) ?? [], payloadRequired)
	);
	let scopes = $derived(
		$permissions
			.filter(
				(s) =>
					$authMode === 'dev' ||
					s.permissions.some(
						(p) => p === '*' || p === 'create' || p === `create:${chosen}`
					)
			)
			.map((s) => s.name)
	);
	let scope = $derived(scopes.includes(scopeChoice) ? scopeChoice : (scopes[0] ?? ''));
	let slug = $derived(slugify(appName));
	let exists = $derived(!!slug && $apps.some((a) => a.name === slug));
	let matched = $derived(slug ? $apps.find((a) => a.name === slug) : undefined);
	let presetFor = '';
	// Fields the user edited; a preset from a matching app leaves them alone.
	let touched = { scope: false, ttl: false, auth: false };
	let parsedTtl = $derived(parseTtl(ttl));
	// An existing app's directory may already hold the required files (the server decides).
	let blocked = $derived(
		(missing.length > 0 && !matched) ||
			(!!upload && upload.mapped.rejected.length + upload.mapped.unmatched.length > 0) ||
			parsedTtl === null
	);

	// Updating starts from the app's settings, applied once when a match first appears.
	$effect(() => {
		if (!matched || presetFor === matched.name) return;
		presetFor = matched.name;
		const st = matched.settings;
		if (!st) return;
		if (st.scopes?.[0] && !touched.scope) scopeChoice = st.scopes[0];
		const t = st.time_to_live;
		if (!touched.ttl)
			ttl = t === 'Forever' ? 'forever' : 'Days' in t ? `${t.Days}d` : `${t.Hours}h`;
		if (!touched.auth) [authUser, authPass] = st.basic_auth ?? ['', ''];
	});

	function choose(name: string) {
		chosen = name;
		const props = blueprints.find((b) => b.id === name)?.template?.parameters?.properties ?? {};
		params = Object.fromEntries(
			Object.entries(props).map(([k, p]) => [
				k,
				p.default ?? (p.type === 'boolean' ? false : undefined)
			])
		);
		imageChoice = '';
		upload = null;
		uploadError = null;
	}

	onMount(async () => {
		setTitle('Create app');
		loadApps().catch(() => {}); // the name-in-use warning only; the form works without it
		try {
			const result = (await authenticatedApiCall('blueprints')) as BlueprintsResponse;
			// The map key is what the server looks blueprints up by; `name` is only for display.
			blueprints = Object.entries(result.blueprints)
				.filter(([, b]) => b.templated)
				.map(([id, b]) => ({ ...b, id }));
			const wanted = page.url.searchParams.get('blueprint');
			choose(blueprints.some((b) => b.id === wanted) ? wanted! : (blueprints[0]?.id ?? ''));
		} catch (err) {
			loadError = err instanceof Error ? err.message : String(err);
		}
	});

	async function pick(input: HTMLInputElement, source: 'zip' | 'folder') {
		const files = input.files;
		if (!files?.length) return;
		upload = null;
		uploadError = null;
		try {
			const entries = source === 'zip' ? await readZip(files[0]) : await readFolder(files);
			const label =
				source === 'zip' ? files[0].name : files[0].webkitRelativePath.split('/')[0];
			upload = {
				label,
				entries,
				mapped: mapPayloadPaths(Object.keys(entries), patterns, source)
			};
		} catch (err) {
			uploadError = `Could not read the ${source}: ${err instanceof Error ? err.message : err}`;
		}
		input.value = ''; // picking the same file again re-reads it
	}

	/** `null` for anything but `<n>h`, `<n>d` or `forever`, letter case ignored. */
	function parseTtl(s: string): AppTtl | null {
		const v = s.trim().toLowerCase();
		if (v === 'forever') return 'Forever';
		const m = /^([1-9]\d*)([hd])$/.exec(v);
		if (!m) return null;
		return m[2] === 'h' ? { Hours: Number(m[1]) } : { Days: Number(m[1]) };
	}

	/** Typed values; empty optional fields are left out so the blueprint default applies. */
	function blueprintParameters(): Record<string, string | number | boolean> {
		const out: Record<string, string | number | boolean> = {};
		for (const [key, p] of properties) {
			const v = params[key];
			if (v === undefined || v === null || v === '') continue;
			out[key] =
				p.type === 'boolean'
					? !!v
					: p.type === 'number' || p.type === 'integer'
						? Number(v)
						: String(v);
		}
		return out;
	}

	async function onsubmit(e: SubmitEvent) {
		e.preventDefault();
		if (!blueprint || !parsedTtl || blocked || submitting) return;
		const inexact = !!appName && !slugifyIsExact(appName);
		if (
			(exists || inexact) &&
			!(await askConfirm({
				title: `Update ${slug}?`,
				body: exists
					? `An app named ${slug} already exists. Creating it from ${blueprint.name} updates that app; the scope, TTL, basic auth and files from this form replace its own.`
					: `The server may normalise "${appName}" differently than this page (${slug}). If an app with the server's name exists, it is updated and the scope, TTL, basic auth and files from this form replace its own.`,
				confirmLabel: 'Update app'
			}))
		)
			return;

		submitting = true;
		submitError = null;
		try {
			const request: CreateAppRequest = {
				app_name: appName,
				settings: {
					public_services: [],
					domain: '',
					environment: {},
					registry: null,
					notify: [],
					middlewares: [],
					disallow_robots: true,
					destroy_on_ttl: false,
					app_blueprint: blueprint.id,
					time_to_live: parsedTtl,
					basic_auth: authUser && authPass ? [authUser, authPass] : null,
					scopes: [scope]
				},
				files: {
					files: upload ? await encodeFiles(upload.entries, upload.mapped.files) : []
				},
				custom_domains: [],
				requested_scopes: [scope],
				blueprint_parameters: blueprintParameters(),
				...(images.length > 1 ? { image } : {})
			};
			const body = JSON.stringify(request);
			const bytes = new TextEncoder().encode(body).length;
			if (
				bytes > MAX_BODY &&
				!(await askConfirm({
					title: 'Send a large upload?',
					body: `The request is ${Math.round(bytes / 1024 / 1024)} MB. Scotty usually accepts up to 50 MB and may reject it.`,
					confirmLabel: 'Send anyway'
				}))
			)
				return;
			const result = (await authenticatedApiCall('apps/create', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body
			})) as RunningAppContext;
			goto(resolve('/tasks/[slug]', { slug: result.task.id }));
		} catch (err) {
			submitError = err instanceof Error ? err.message : String(err);
		} finally {
			submitting = false;
		}
	}
</script>

<div class="ph">
	<div class="ph-row"><h1>Create app</h1></div>
	<p class="lead">
		Create an app from a blueprint: fill in its parameters and upload the files it serves.
		Creating an app that already exists updates it.
	</p>
</div>

{#if loadError}
	<div class="banner err">
		<Icon icon="ph:warning-circle" class="ic" aria-hidden="true" />
		<div class="txt"><b>Could not load the blueprints</b><span>{loadError}</span></div>
	</div>
{:else if blueprints.length}
	<form class="sec create" {onsubmit}>
		<fieldset class="scopes">
			<legend>Blueprint</legend>
			{#each blueprints as b (b.id)}
				<label>
					<input
						type="radio"
						name="blueprint"
						value={b.id}
						checked={chosen === b.id}
						onchange={() => choose(b.id)}
					/>
					<span>{b.name}<span class="mono">{b.description}</span></span>
				</label>
			{/each}
		</fieldset>

		<div class="grid">
			<label for="app-name">App name</label>
			<div>
				<input id="app-name" bind:value={appName} required autocomplete="off" />
				{#if slug && slug !== appName}<p class="help">
						Created as <span class="mono">{slug}</span>
					</p>{/if}
				{#if exists}
					<p class="help warn">
						An app named <span class="mono">{slug}</span> exists; creating updates it.
					</p>
				{/if}
			</div>

			<label for="scope">Scope</label>
			<div>
				<select
					id="scope"
					value={scope}
					onchange={(e) => {
						scopeChoice = e.currentTarget.value;
						touched.scope = true;
					}}
					required
				>
					{#each scopes as s (s)}<option value={s}>{s}</option>{/each}
				</select>
				{#if !scopes.length}<p class="help warn">
						You may not create apps from this blueprint in any scope.
					</p>{/if}
			</div>

			{#if images.length > 1}
				<label for="image">Image</label>
				<select
					id="image"
					value={image}
					onchange={(e) => (imageChoice = e.currentTarget.value)}
				>
					{#each images as i (i)}<option value={i}>{i}</option>{/each}
				</select>
			{/if}

			{#each properties as [key, p] (key)}
				<label for="param-{key}">{p.title ?? key}{required.has(key) ? ' *' : ''}</label>
				<div>
					{#if p.enum}
						<select
							id="param-{key}"
							bind:value={params[key]}
							required={required.has(key)}
						>
							{#each p.enum as v (v)}<option value={v}>{v}</option>{/each}
						</select>
					{:else if p.type === 'boolean'}
						<input
							id="param-{key}"
							type="checkbox"
							bind:checked={params[key] as boolean}
						/>
					{:else if p.type === 'string'}
						<input
							id="param-{key}"
							bind:value={params[key]}
							required={required.has(key)}
						/>
					{:else}
						<input
							id="param-{key}"
							type="number"
							step={p.type === 'integer' ? 1 : 'any'}
							bind:value={params[key]}
							required={required.has(key)}
						/>
					{/if}
					{#if p.description}<p class="help">{p.description}</p>{/if}
				</div>
			{/each}

			<label for="ttl">Time to live</label>
			<div>
				<input
					id="ttl"
					bind:value={ttl}
					oninput={() => (touched.ttl = true)}
					required
					pattern="[1-9][0-9]*[hdHD]|[Ff][Oo][Rr][Ee][Vv][Ee][Rr]"
				/>
				<p class="help">
					Hours (<span class="mono">12h</span>), days (<span class="mono">7d</span>) or
					<span class="mono">forever</span>.
				</p>
			</div>

			<label for="auth-user">Basic auth</label>
			<div class="pair">
				<input
					id="auth-user"
					bind:value={authUser}
					oninput={() => (touched.auth = true)}
					placeholder="User"
					aria-label="Basic auth user"
					required={!!authPass}
					autocomplete="off"
				/>
				<input
					type="password"
					bind:value={authPass}
					oninput={() => (touched.auth = true)}
					placeholder="Password"
					aria-label="Basic auth password"
					required={!!authUser}
					autocomplete="new-password"
				/>
			</div>

			{#if patterns.length}
				<span>Files</span>
				<div>
					<div class="acts">
						<label class="btn outline">
							<Icon icon="ph:file-zip" class="ic sm" aria-hidden="true" />Zip file
							<input
								class="sr-only"
								type="file"
								accept=".zip,application/zip"
								onchange={(e) => pick(e.currentTarget, 'zip')}
							/>
						</label>
						<label class="btn outline">
							<Icon icon="ph:folder" class="ic sm" aria-hidden="true" />Folder
							<input
								class="sr-only"
								type="file"
								webkitdirectory
								onchange={(e) => pick(e.currentTarget, 'folder')}
							/>
						</label>
						{#if upload}<span class="meta"
								>{upload.label}: {upload.mapped.files.length} files</span
							>{/if}
					</div>
					<p class="help">Pick the folder that contains your site.</p>
					<p class="help">
						Accepted paths: <span class="mono">{patterns.join(', ')}</span>
					</p>
					{#if payloadRequired.length}
						<p class="help">
							Required: <span class="mono">{payloadRequired.join(', ')}</span>
						</p>
					{/if}
					{#if uploadError}<p class="help err">{uploadError}</p>{/if}
					{#if upload?.mapped.rejected.length}
						<p class="help err">
							Compose and <span class="mono">.env</span> files are not allowed:
							<span class="mono">{upload.mapped.rejected.join(', ')}</span>
						</p>
					{/if}
					{#if upload?.mapped.unmatched.length}
						<p class="help err">
							Not accepted by the blueprint: <span class="mono"
								>{upload.mapped.unmatched.join(', ')}</span
							>
						</p>
					{/if}
					{#if matched && missing.length}
						<p class="help warn">
							<span class="mono">{missing.join(', ')}</span> must already exist in the app.
						</p>
					{:else if upload && missing.length}
						<p class="help err">
							Required but missing: <span class="mono">{missing.join(', ')}</span>
						</p>
					{/if}
				</div>
			{/if}
		</div>

		{#if submitError}
			<div class="banner err">
				<Icon icon="ph:warning-circle" class="ic" aria-hidden="true" />
				<div class="txt">
					<b>Scotty did not create the app</b><span>{submitError}</span>
				</div>
			</div>
		{/if}

		<div class="acts">
			<button
				type="submit"
				class="btn filled primary"
				disabled={submitting || blocked || !scope}
			>
				{#if submitting}
					<Icon icon="ph:spinner" class="ic sm spin" aria-hidden="true" />
				{:else}
					<Icon icon="ph:rocket-launch" class="ic sm" aria-hidden="true" />
				{/if}
				Create app
			</button>
			<a class="btn outline" href={resolve('/dashboard')}>Cancel</a>
		</div>
	</form>
{:else}
	<div class="empty">
		<h2>No blueprint to create from</h2>
		<p>None of the blueprints you may use can create an app from the dashboard.</p>
	</div>
{/if}

<style>
	.create {
		max-width: 760px;
	}
	.grid {
		display: grid;
		grid-template-columns: minmax(120px, 180px) minmax(0, 1fr);
		gap: 16px;
		align-items: start;
		font: var(--text-app-body-sm);
	}
	.grid > label,
	.grid > span {
		font-weight: 600;
		padding-top: 8px;
	}
	.grid input:not([type='checkbox']):not([type='file']),
	.grid select {
		height: var(--app-control-height);
		padding: 0 12px;
		border: 1px solid var(--color-app-border-strong);
		border-radius: var(--border-radius);
		background: none;
		color: inherit;
		font: inherit;
		width: 100%;
		max-width: 360px;
	}
	.grid input[type='checkbox'] {
		margin-top: 12px;
	}
	.pair {
		display: flex;
		gap: 8px;
		flex-wrap: wrap;
	}
	.pair input {
		flex: 1;
		min-width: 140px;
	}
	.help {
		margin: 4px 0 0;
		color: var(--color-copy-secondary);
	}
	.help.warn {
		color: var(--color-status-warning);
	}
	.help.err {
		color: var(--color-status-error);
	}
	.acts label.btn {
		cursor: pointer;
	}
</style>
