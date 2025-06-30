<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { ArrowLeft } from '@lucide/svelte';
	import type { UINode } from '$lib/types';
	import NodeRenderer from '../NodeRenderer.svelte';
	import LoadingIndicator from '../LoadingIndicator.svelte';
	import HeaderInput from '../HeaderInput.svelte';
	import Icon from '../Icon.svelte';

	type Props = {
		rootNode: UINode | undefined;
		searchText: string;
		onPopView: () => void;
		onDispatch: (instanceId: number, handlerName: string, args: unknown[]) => void;
		uiTree: Map<number, UINode>;
		showBackButton: boolean;
		inputRef?: HTMLInputElement | null;
	};

	let {
		rootNode,
		searchText = $bindable(),
		onPopView,
		onDispatch,
		uiTree,
		showBackButton,
		inputRef = $bindable()
	}: Props = $props();

	const viewType = $derived(rootNode?.type);
	const placeholder = $derived((rootNode?.props.searchBarPlaceholder as string) ?? 'Search...');
	const searchBarAccessoryId = $derived(rootNode?.namedChildren?.searchBarAccessory);
	const isLoading = $derived((rootNode?.props.isLoading as boolean) ?? false);
</script>

<header class="relative mb-2 flex h-15 shrink-0 items-center pr-4 pl-[18px]">
	{#if showBackButton}
		<Button size="icon" onclick={onPopView} variant="secondary" class="size-6">
			<Icon icon="arrow-left-16" />
		</Button>
	{/if}

	<div class="flex flex-grow items-center">
		{#if viewType === 'List' || viewType === 'Grid'}
			<HeaderInput
				{placeholder}
				bind:value={searchText}
				bind:ref={inputRef}
				autofocus
				class="!pl-2.5"
			/>
		{:else if viewType === 'Form'}
			<div class="grow"></div>
		{/if}
		{#if searchBarAccessoryId && (viewType === 'List' || viewType === 'Grid' || viewType === 'Form')}
			{#key searchBarAccessoryId}
				<div class="mx-2">
					<NodeRenderer nodeId={searchBarAccessoryId} {uiTree} {onDispatch} />
				</div>
			{/key}
		{/if}
	</div>

	<LoadingIndicator {isLoading} />
</header>
