<script lang="ts">
	import { setContext, tick } from 'svelte';
	import MainLayout from '$lib/components/layout/MainLayout.svelte';
	import Header from '$lib/components/layout/Header.svelte';
	import Content from '$lib/components/layout/Content.svelte';
	import { uiStore } from '$lib/ui.svelte';
	import path from 'path';
	import ActionBar from './nodes/shared/ActionBar.svelte';
	import Footer from './layout/Footer.svelte';
	import NodeRenderer from './NodeRenderer.svelte';
	import { focusManager } from '$lib/focus.svelte';
	import { SvelteMap } from 'svelte/reactivity';
	import { sidecarService } from '$lib/sidecar.svelte';

	const {
		uiTree,
		rootNodeId,
		selectedNodeId,
		toasts,
		currentRunningPlugin,
		primaryAction: primaryActionObject,
		secondaryAction,
		actionPanel,
		allActions
	} = $derived(uiStore);

	type Props = {
		onPopView: () => void;
		onToastAction: (toastId: number, actionType: 'primary' | 'secondary') => void;
	};

	let { onPopView, onToastAction }: Props = $props();

	const rootNode = $derived(uiTree.get(rootNodeId!));
	const selectedItemNode = $derived(uiTree.get(selectedNodeId!));
	let searchText = $state('');
	let searchInputEl: HTMLInputElement | null = $state(null);
	const navigationTitle = $derived(rootNode?.props.navigationTitle as string | undefined);
	const toastToShow = $derived(Array.from(toasts.entries()).sort((a, b) => b[0] - a[0])[0]?.[1]);
	const showActionPanelDropdown = $derived((allActions?.length ?? 0) > 1);
	const formValues = new SvelteMap<string, unknown>();

	const assetsPath = $derived(
		currentRunningPlugin ? path.dirname(currentRunningPlugin.pluginPath) + '/assets' : ''
	);
	setContext('assetsPath', assetsPath);
	setContext('form-context', {
		register: (fieldId: string, value: unknown) => {
			formValues.set(fieldId, value);
		}
	});

	function handleSelect(nodeId: number | undefined) {
		uiStore.selectedNodeId = nodeId;
	}

	function handleDispatch(instanceId: number, handlerName: string, args: unknown[]) {
		const instance = uiTree.get(instanceId);
		if (instance?.type === 'Action.SubmitForm' && handlerName === 'onSubmit') {
			const valuesObject = Object.fromEntries(formValues.entries());
			sidecarService.dispatchEvent('dispatch-event', {
				instanceId,
				handlerName,
				args: [valuesObject]
			});
			formValues.clear();
		} else {
			sidecarService.dispatchEvent('dispatch-event', { instanceId, handlerName, args });
		}
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape' && !event.defaultPrevented) {
			if (event.defaultPrevented) {
				return;
			}
			onPopView();
			return;
		}
	}

	$effect(() => {
		if (focusManager.activeScope === 'main-input') {
			tick().then(() => {
				searchInputEl?.focus();
			});
		}
	});

	$effect(() => {
		if (rootNode) {
			handleDispatch(rootNode.id, 'onSearchTextChange', [searchText]);
		}
	});
</script>

<svelte:window onkeydown={handleKeydown} />

{#if rootNode}
	<MainLayout>
		{#snippet header()}
			<Header
				{rootNode}
				bind:searchText
				bind:inputRef={searchInputEl}
				{onPopView}
				onDispatch={handleDispatch}
				{uiTree}
				showBackButton={true}
			/>
		{/snippet}

		{#snippet content()}
			<Content
				{rootNode}
				{selectedItemNode}
				{uiTree}
				onDispatch={handleDispatch}
				onSelect={handleSelect}
				{searchText}
			/>
		{/snippet}

		{#snippet footer()}
			{#if toastToShow}
				<Footer toast={toastToShow} {onToastAction} />
			{:else}
				<ActionBar title={navigationTitle}>
					{#snippet primaryAction({ props })}
						{#if primaryActionObject}
							<NodeRenderer
								{...props}
								nodeId={primaryActionObject.id}
								{uiTree}
								onDispatch={handleDispatch}
								displayAs="button"
							/>
						{/if}
					{/snippet}
					{#snippet actions()}
						{#if showActionPanelDropdown && actionPanel}
							<NodeRenderer nodeId={actionPanel.id} {uiTree} onDispatch={handleDispatch} />
						{/if}
					{/snippet}
				</ActionBar>
			{/if}
		{/snippet}
	</MainLayout>
{/if}
