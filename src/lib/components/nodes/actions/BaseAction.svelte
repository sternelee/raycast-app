<script lang="ts">
	import type { KeyboardShortcut as KeyboardShortcutType } from '$lib/props/actions';
	import { DropdownMenuItem, DropdownMenuShortcut } from '$lib/components/ui/dropdown-menu';
	import { Button } from '$lib/components/ui/button';
	import type { ImageLike } from '$lib/props';
	import Icon from '$lib/components/Icon.svelte';
	import { Kbd } from '$lib/components/ui/kbd';
	import KeyboardShortcut from '$lib/components/KeyboardShortcut.svelte';

	type Props = {
		title: string;
		shortcut?: KeyboardShortcutType | null;
		icon?: ImageLike;
		isPrimaryAction?: boolean;
		isSecondaryAction?: boolean;
		displayAs?: 'item' | 'button';
		style?: 'regular' | 'destructive';
		onclick: (event: MouseEvent) => void;
	};

	let {
		title,
		shortcut = undefined,
		icon,
		isPrimaryAction = false,
		isSecondaryAction = false,
		displayAs = 'item',
		style = 'regular',
		onclick
	}: Props = $props();
</script>

{#if displayAs === 'button'}
	<Button
		{onclick}
		variant="ghost"
		size="sm"
		class={style === 'destructive' ? 'text-destructive' : ''}
	>
		{title}
		<Kbd>⏎</Kbd>
	</Button>
{:else}
	<DropdownMenuItem
		class="rounded-md p-2 text-left {style === 'destructive'
			? 'text-destructive focus:text-destructive-foreground focus:bg-destructive'
			: ''}"
		{onclick}
	>
		{#if icon}
			<Icon {icon} class="size-4" />
		{/if}
		{title}
		{#if isPrimaryAction}
			<DropdownMenuShortcut>
				<KeyboardShortcut shortcut={{ key: 'enter', modifiers: [] }} />
			</DropdownMenuShortcut>
		{:else if isSecondaryAction}
			<DropdownMenuShortcut>
				<KeyboardShortcut shortcut={{ key: 'enter', modifiers: ['ctrl'] }} />
			</DropdownMenuShortcut>
		{:else if shortcut}
			<DropdownMenuShortcut>
				<KeyboardShortcut {shortcut} />
			</DropdownMenuShortcut>
		{/if}
	</DropdownMenuItem>
{/if}
