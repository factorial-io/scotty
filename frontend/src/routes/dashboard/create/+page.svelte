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

	/** Client hint only (design D5); the server's `api.create_app_max_size` decides. */
	const MAX_BODY = 50 * 1024 * 1024;

	type Value = string | number | boolean | null | undefined;

	let blueprints = $state<Blueprint[]>([]);
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

	let blueprint = $derived(blueprints.find((b) => b.name === chosen));
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
	let blocked = $derived(
		missing.length > 0 ||
			(!!upload && upload.mapped.rejected.length + upload.mapped.unmatched.length > 0)
	);

	// Updating starts from the app's settings, applied once when a match first appears.
	$effect(() => {
		if (!matched || presetFor === matched.name) return;
		presetFor = matched.name;
		const st = matched.settings;
		if (!st) return;
		if (st.scopes?.[0]) scopeChoice = st.scopes[0];
		const t = st.time_to_live;
		ttl = t === 'Forever' ? 'forever' : 'Days' in t ? `${t.Days}d` : `${t.Hours}h`;
		[authUser, authPass] = st.basic_auth ?? ['', ''];
	});

	function choose(name: string) {
		chosen = name;
		const props =
			blueprints.find((b) => b.name === name)?.template?.parameters?.properties ?? {};
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
			blueprints = Object.values(result.blueprints).filter((b) => b.templated);
			const wanted = page.url.searchParams.get('blueprint');
			choose(
				blueprints.some((b) => b.name === wanted) ? wanted! : (blueprints[0]?.name ?? '')
			);
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

	function parseTtl(s: string): AppTtl {
		const m = /^(\d+)([hd])$/i.exec(s.trim());
		if (!m) return 'Forever';
		return m[2].toLowerCase() === 'h' ? { Hours: Number(m[1]) } : { Days: Number(m[1]) };
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
		if (!blueprint || blocked || submitting) return;
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
					app_blueprint: blueprint.name,
					time_to_live: parseTtl(ttl),
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
			if (
				body.length > MAX_BODY &&
				!(await askConfirm({
					title: 'Send a large upload?',
					body: `The request is ${Math.round(body.length / 1024 / 1024)} MB. Scotty usually accepts up to 50 MB and may reject it.`,
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
			{#each blueprints as b (b.name)}
				<label>
					<input
						type="radio"
						name="blueprint"
						value={b.name}
						checked={chosen === b.name}
						onchange={() => choose(b.name)}
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
					onchange={(e) => (scopeChoice = e.currentTarget.value)}
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
				<input id="ttl" bind:value={ttl} required pattern="[1-9][0-9]*[hd]|forever" />
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
					placeholder="User"
					aria-label="Basic auth user"
					required={!!authPass}
					autocomplete="off"
				/>
				<input
					type="password"
					bind:value={authPass}
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
					{#if upload && missing.length}
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
