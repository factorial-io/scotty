<script lang="ts">
	import Icon from '@iconify/svelte';
	import { DropdownMenu } from 'bits-ui';
	import { authMode, userInfo } from '../stores/sessionStore';
	import { authService } from '../lib/authService';
	import { getGravatarUrl, getUserInitials } from '#lib/gravatar.ts';
	import { theme, type ThemeChoice } from '../stores/themeStore';

	let { version = '' }: { version?: string } = $props();

	let name = $derived(
		$authMode === 'oauth'
			? $userInfo?.name || $userInfo?.id || 'Unknown user'
			: $authMode === 'dev'
				? 'Dev user'
				: 'User'
	);
	let method = $derived(
		$authMode === 'oauth'
			? 'Signed in via OAuth'
			: $authMode === 'bearer'
				? 'Signed in with a bearer token'
				: 'Development mode, no sign-in'
	);

	// IdP picture first, then Gravatar (d=404 so a missing one falls back to the initials).
	let picture = $derived(
		$userInfo?.picture || ($userInfo?.email ? getGravatarUrl($userInfo.email, 64, '404') : '')
	);
	let pictureFailed = $state(false);
	$effect(() => {
		void picture;
		pictureFailed = false;
	});

	const themes: { value: ThemeChoice; label: string; icon: string }[] = [
		{ value: 'light', label: 'Light', icon: 'ph:sun' },
		{ value: 'dark', label: 'Dark', icon: 'ph:moon' },
		{ value: 'system', label: 'System', icon: 'ph:monitor' }
	];
</script>

<div class="user">
	<div class="av" aria-hidden="true">
		{#if picture && !pictureFailed}
			<img
				src={picture}
				alt=""
				referrerpolicy="no-referrer"
				onerror={() => (pictureFailed = true)}
			/>
		{:else}
			{getUserInitials(name, $userInfo?.email)}
		{/if}
	</div>
	<div class="who">
		<b>{name}</b><span>Scotty v{version}</span>
	</div>
	<DropdownMenu.Root>
		<DropdownMenu.Trigger class="ibtn" aria-label="Account menu">
			<Icon icon="ph:caret-up-down" class="ic" aria-hidden="true" />
		</DropdownMenu.Trigger>
		<DropdownMenu.Portal>
			<DropdownMenu.Content class="menu" side="top" align="end" sideOffset={8}>
				<div class="hd">{method}</div>
				{#if $userInfo?.email}<div class="sub" style="padding: 0 10px 6px">
						{$userInfo.email}
					</div>{/if}
				<DropdownMenu.Separator />
				<DropdownMenu.RadioGroup
					value={$theme}
					onValueChange={(v) => theme.choose(v as ThemeChoice)}
					aria-label="Theme"
				>
					{#each themes as t (t.value)}
						<DropdownMenu.RadioItem value={t.value} closeOnSelect={false}>
							{#snippet children({ checked })}
								<Icon icon={t.icon} class="ic sm" aria-hidden="true" />
								{t.label}
								{#if checked}<Icon
										icon="ph:check"
										class="ic sm"
										style="margin-left:auto"
										aria-hidden="true"
									/>{/if}
							{/snippet}
						</DropdownMenu.RadioItem>
					{/each}
				</DropdownMenu.RadioGroup>
				<DropdownMenu.Separator />
				<DropdownMenu.Item
					onSelect={() =>
						window.open(
							`https://github.com/factorial-io/scotty/releases/tag/v${version}`,
							'_blank',
							'noopener'
						)}
				>
					<Icon icon="ph:arrow-square-out" class="ic sm" aria-hidden="true" />
					Release notes v{version}
				</DropdownMenu.Item>
				<DropdownMenu.Item onSelect={() => authService.logout('/login')}>
					<Icon icon="ph:sign-out" class="ic sm" aria-hidden="true" />
					Sign out
				</DropdownMenu.Item>
			</DropdownMenu.Content>
		</DropdownMenu.Portal>
	</DropdownMenu.Root>
</div>
