<script lang="ts">
	import ErrorDialog from '../components/error-dialog.svelte';
	import Toasts from '../components/toasts.svelte';
	import ConfirmDialog from '../components/confirm-dialog.svelte';
	import '../app.css';
	import logo from '#lib/assets/scotty.svg';
	import chevron from '#lib/assets/chevron.svg';
	import Icon from '@iconify/svelte';
	import { publicApiCall, initializeAuth } from '#lib';
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import title from '../stores/titleStore';
	import UserInfo from '../components/user-info.svelte';
	import ScottyType from '../components/scotty-type.svelte';
	import NavItem from '../components/nav-item.svelte';
	import { Tooltip } from 'bits-ui';
	import { MediaQuery } from 'svelte/reactivity';
	import WebSocketStatus from '../components/websocket-status.svelte';
	import { webSocketStore } from '../stores/webSocketStore';
	import { isLoggedIn } from '../stores/userStore';
	import { loadUserPermissions } from '../stores/permissionStore';
	import { theme } from '../stores/themeStore';
	import { siteInfo as site } from '../stores/siteStore';
	import { applySuisseFonts } from '../lib/fonts';
	import { apps, loadApps } from '../stores/appsStore';
	import { runningTaskCount } from '../stores/tasksStore';
	import { isSearchShortcut } from '../lib/shortcut';
	import { connectionCount, refreshConnectionCount } from '../lib/connections';

	let { children } = $props();

	// Load permissions and the connection count as soon as the user is (or
	// becomes) logged in, so permission-gated UI works on a hard page load too.
	$effect(() => {
		if ($isLoggedIn) {
			loadUserPermissions();
			refreshConnectionCount();
			// The sidebar counts all apps, also when a page only loaded one.
			loadApps();
		}
	});

	let collapsed = $state(false);
	const narrow = new MediaQuery('max-width: 860px');
	let rail = $derived(collapsed || narrow.current);

	// Pages that stand on their own, without the sidebar and top bar.
	const STANDALONE = ['/login', '/oauth', '/device', '/landing'];
	let standalone = $derived(STANDALONE.some((p) => page.url.pathname.startsWith(p)));

	let section = $derived(
		page.url.pathname.startsWith('/tasks')
			? 'tasks'
			: page.url.pathname.startsWith('/connections')
				? 'connections'
				: 'apps'
	);

	// Breadcrumbs from the route: Apps › app › service, Tasks › task.
	let crumbs = $derived.by(() => {
		const parts = page.url.pathname.split('/').filter(Boolean).map(decodeURIComponent);
		const [root, ...rest] = parts;
		const base =
			root === 'tasks'
				? { label: 'Tasks', href: '/tasks' }
				: root === 'connections'
					? { label: 'Connections', href: '/connections' }
					: { label: 'Apps', href: '/dashboard' };
		const list = [base];
		let href = base.href;
		for (const p of rest) {
			href += '/' + encodeURIComponent(p);
			list.push({ label: p, href });
		}
		return list;
	});

	function findApp() {
		goto(resolve('/dashboard')).then(() =>
			document.querySelector<HTMLInputElement>('#app-search')?.focus()
		);
	}

	function onKeydown(e: KeyboardEvent) {
		if (standalone || !isSearchShortcut(e)) return;
		e.preventDefault();
		findApp();
	}

	onMount(async () => {
		theme.init();
		await initializeAuth();
		// Connects once the user is logged in.
		webSocketStore.initialize();
		const info = (await publicApiCall('info')) as typeof $site;
		site.set(info);
		applySuisseFonts(info.suisse_fonts_url);
	});
</script>

<svelte:head>
	<title>{$title}</title>
</svelte:head>
<svelte:window onkeydown={onKeydown} />

{#if standalone}
	{@render children()}
{:else}
	<div class="shell" class:collapsed>
		<aside class="side">
			<a class="org" href={resolve('/dashboard')} title="Scotty @ {$site.domain}">
				<span class="logo-tile"><img src={logo} alt="Scotty" /></span>
				<span class="brand"><ScottyType /><span class="inst">{$site.domain}</span></span>
			</a>
			<Tooltip.Provider>
				<nav aria-label="Main">
					<div class="grp">Workspace</div>
					<NavItem
						href={resolve('/dashboard')}
						icon="ph:squares-four"
						label="Apps"
						active={section === 'apps'}
						{rail}
					>
						{#snippet count()}<span class="count">{$apps.length || ''}</span>{/snippet}
					</NavItem>
					<NavItem
						href={resolve('/tasks')}
						icon="ph:list-checks"
						label="Tasks"
						active={section === 'tasks'}
						{rail}
					>
						{#snippet count()}<span class="count" class:urgent={$runningTaskCount > 0}
								>{$runningTaskCount ? `${$runningTaskCount} running` : ''}</span
							>{/snippet}
					</NavItem>
					<div class="grp">Access</div>
					<NavItem
						href={resolve('/connections')}
						icon="ph:plugs-connected"
						label="Connections"
						active={section === 'connections'}
						{rail}
					>
						{#snippet count()}<span class="count">{$connectionCount ?? ''}</span
							>{/snippet}
					</NavItem>
					<NavItem
						href="/rapidoc"
						icon="ph:book-open-text"
						label="API documentation"
						external
						{rail}
					>
						{#snippet count()}<span class="count ext"
								><Icon
									icon="ph:arrow-square-out"
									class="ic sm"
									aria-hidden="true"
								/></span
							>{/snippet}
					</NavItem>
				</nav>
			</Tooltip.Provider>
			<UserInfo version={$site.version} />
		</aside>

		<div class="main">
			<header class="top">
				<button
					class="ibtn"
					id="sideToggle"
					aria-label="Toggle sidebar"
					aria-expanded={!collapsed}
					onclick={() => (collapsed = !collapsed)}
				>
					<Icon icon="ph:sidebar-simple" class="ic" aria-hidden="true" />
				</button>
				<nav class="crumbs" aria-label="Breadcrumb">
					{#each crumbs as c, i (c.href)}
						{#if i > 0}<img src={chevron} alt="" />{/if}
						{#if i < crumbs.length - 1}
							<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
							<a class="link" href={c.href}>{c.label}</a>
						{:else}
							<span aria-current="page">{c.label}</span>
						{/if}
					{/each}
				</nav>
				<div class="right">
					<WebSocketStatus />
					<button class="searchbtn" onclick={findApp}>
						<Icon icon="ph:magnifying-glass" class="ic sm" aria-hidden="true" />
						<span class="label">Find an app</span><kbd>/</kbd>
					</button>
				</div>
			</header>
			<main class="page" tabindex="-1">
				<div class="page-inner">
					{@render children()}
				</div>
			</main>
		</div>
	</div>
{/if}
<ErrorDialog />
<Toasts />
<ConfirmDialog />
