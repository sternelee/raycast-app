<script lang="ts">
	import { writeText } from '@tauri-apps/plugin-clipboard-manager';
	import { openUrl } from '@tauri-apps/plugin-opener';
	import type { UINode } from '$lib/types';
	import { useTypedNode } from '$lib/node.svelte';
	import { actionBus } from '$lib/actions.svelte';
	import BaseAction from './BaseAction.svelte';
	import type { ActionCopyToClipboardProps, ActionOpenInBrowserProps } from '$lib/props';
	import { uiStore } from '$lib/ui.svelte';

	type Props = {
		nodeId: number;
		uiTree: Map<number, UINode>;
		onDispatch: (instanceId: number, handlerName: string, args: unknown[]) => void;
		displayAs?: 'item' | 'button';
	};

	let { nodeId, uiTree, onDispatch, displayAs = 'item' }: Props = $props();

	const { node, props: componentProps } = $derived.by(
		useTypedNode(() => ({
			nodeId,
			uiTree,
			type: [
				'Action',
				'Action.Push',
				'Action.CopyToClipboard',
				'Action.OpenInBrowser',
				'Action.SubmitForm'
			]
		}))
	);

	const isPrimaryAction = $derived.by(() => uiStore.primaryAction?.id === nodeId);
	const isSecondaryAction = $derived.by(() => uiStore.secondaryAction?.id === nodeId);

	const title = $derived.by(() => {
		if (!node || !componentProps) return '';

		switch (node.type) {
			case 'Action.CopyToClipboard':
				return componentProps.title ?? 'Copy to Clipboard';
			case 'Action.OpenInBrowser':
				return componentProps.title ?? 'Open in Browser';
			default:
				return componentProps.title ?? '';
		}
	});

	function handleClick() {
		if (!node || !componentProps) return;

		switch (node.type) {
			case 'Action.CopyToClipboard':
				const copyProps = componentProps as ActionCopyToClipboardProps;
				writeText(copyProps.content);
				onDispatch(nodeId, 'onCopy', []);
				break;

			case 'Action.OpenInBrowser':
				const openProps = componentProps as ActionOpenInBrowserProps;
				openUrl(openProps.url);
				onDispatch(nodeId, 'onOpenInBrowser', []);
				break;

			case 'Action.SubmitForm':
				onDispatch(nodeId, 'onSubmit', []);
				break;

			case 'Action.Push':
			case 'Action':
			default:
				onDispatch(nodeId, 'onAction', []);
				break;
		}
	}

	$effect(() => {
		// TODO: is this the best way to check if we're in a dropdown?
		if (isPrimaryAction && displayAs === 'button') {
			actionBus.registerPrimary(handleClick);
			return () => actionBus.unregisterPrimary();
		}
	});
	$effect(() => {
		if (isSecondaryAction && displayAs === 'button') {
			actionBus.registerSecondary(handleClick);
			return () => actionBus.unregisterSecondary();
		}
	});
</script>

{#if componentProps}
	<BaseAction
		{title}
		icon={componentProps.icon}
		shortcut={componentProps.shortcut}
		{isPrimaryAction}
		{isSecondaryAction}
		{displayAs}
		onclick={handleClick}
	/>
{/if}
