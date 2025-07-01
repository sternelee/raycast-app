<script lang="ts">
	import type { UINode } from '$lib/types';
	import type { GridItemProps } from '$lib/props';
	import GridSection from './GridSection.svelte';
	import GridItem from './GridItem.svelte';
	import { useGridView } from '$lib/views';
	import { useTypedNode } from '$lib/node.svelte';
	import { VList, type VListHandle } from 'virtua/svelte';

	type Props = {
		nodeId: number;
		uiTree: Map<number, UINode>;
		onSelect: (nodeId: number | undefined) => void;
		searchText: string;
	};
	let { nodeId, uiTree, onSelect, searchText }: Props = $props();

	const { props: gridProps } = $derived.by(useTypedNode(() => ({ nodeId, uiTree, type: 'Grid' })));

	const view = useGridView(() => ({
		nodeId,
		uiTree,
		onSelect,
		columns: gridProps?.columns ?? 6,
		searchText,
		filtering: gridProps?.filtering,
		onSearchTextChange: !!gridProps?.onSearchTextChange,
		inset: gridProps?.inset
	}));

	let vlist: VListHandle | null = null;
	$effect(() => {
		view.vlistInstance = vlist ?? undefined;
	});
</script>

<svelte:window onkeydown={view.handleKeydown} />

<div class="flex h-full flex-col">
	<div class="flex-grow overflow-y-auto px-4">
		<VList bind:this={vlist} data={view.virtualListItems} getKey={(item) => item.id} class="h-full">
			{#snippet children(item)}
				<div class="h-2"></div>
				{#if item.type === 'header'}
					<GridSection props={item.props} />
				{:else if item.type === 'row'}
					<div
						class="grid content-start gap-x-2.5"
						style:grid-template-columns={`repeat(${gridProps?.columns ?? 6}, 1fr)`}
					>
						{#each item.items as gridItem (gridItem.id)}
							{@const flatIndex = view.flatList.findIndex((f) => f.id === gridItem.id)}
							<div id="item-{gridItem.id}">
								<GridItem
									props={gridItem.props as GridItemProps}
									selected={view.selectedItemIndex === flatIndex}
									onclick={() => view.setSelectedItemIndex(flatIndex)}
									inset={gridItem.inset}
								/>
							</div>
						{/each}
					</div>
				{/if}
			{/snippet}
		</VList>
	</div>
</div>
