import { _useBaseView, type BaseViewArgs } from './base.svelte';
import type { GridInset, ViewSectionProps, GridItemProps } from '$lib/props';
import { focusManager } from '$lib/focus.svelte';
import type { VListHandle } from 'virtua/svelte';

export type VirtualGridItem =
	| { id: string | number; type: 'header'; props: ViewSectionProps }
	| {
			id: string;
			type: 'row';
			items: { id: number; type: 'item'; props: GridItemProps; inset?: GridInset }[];
	  };

export function useGridView(args: () => BaseViewArgs & { columns: number; inset?: GridInset }) {
	const base = _useBaseView(args, 'Grid.Item');
	const { columns, inset: gridInset } = $derived.by(args);

	const processedFlatList = $derived.by(() => {
		const list = base.flatList;
		const newList: (typeof list)[number][] = [];
		let currentSectionInset: GridInset | undefined;

		for (const item of list) {
			if (item.type === 'header') {
				const sectionProps = item.props as ViewSectionProps;
				if (item.id === -1) {
					currentSectionInset = gridInset;
				} else {
					currentSectionInset = sectionProps.inset;
				}
				newList.push(item);
			} else {
				newList.push({ ...item, inset: currentSectionInset });
			}
		}
		return newList;
	});

	const virtualListItems = $derived.by((): VirtualGridItem[] => {
		const list: VirtualGridItem[] = [];
		let currentRow: (typeof processedFlatList)[number][] = [];

		for (const item of processedFlatList) {
			if (item.type === 'header') {
				if (currentRow.length > 0) {
					list.push({ id: `row-${list.length}`, type: 'row', items: currentRow });
					currentRow = [];
				}
				list.push({ id: `header-${item.id}`, type: 'header', props: item.props });
			} else if (item.type === 'item') {
				currentRow.push(item);
				if (currentRow.length === columns) {
					list.push({ id: `row-${list.length}`, type: 'row', items: currentRow });
					currentRow = [];
				}
			}
		}
		if (currentRow.length > 0) {
			list.push({ id: `row-${list.length}`, type: 'row', items: currentRow });
		}
		return list;
	});

	const flatIndexToVirtualRowIndexMap = $derived.by(() => {
		const map = new Map<number, number>();
		virtualListItems.forEach((vItem, vIndex) => {
			if (vItem.type === 'row') {
				vItem.items.forEach((item) => {
					const flatIndex = processedFlatList.findIndex((f) => f.id === item.id);
					if (flatIndex !== -1) {
						map.set(flatIndex, vIndex);
					}
				});
			}
		});
		return map;
	});

	let vlistInstance = $state<VListHandle | undefined>();

	$effect(() => {
		if (base.selectedItemIndex >= 0 && vlistInstance) {
			const virtualRowIndex = flatIndexToVirtualRowIndexMap.get(base.selectedItemIndex);
			if (virtualRowIndex !== undefined) {
				vlistInstance.scrollToIndex(virtualRowIndex, { align: 'nearest' });
			}
		}
	});

	const gridMap: {
		flatListIndex: number;
		sectionIndex: number;
		rowIndex: number;
		colIndex: number;
	}[] = $derived.by(() => {
		const newGridMap: {
			flatListIndex: number;
			sectionIndex: number;
			rowIndex: number;
			colIndex: number;
		}[] = [];
		let sectionIndex = -1,
			rowIndex = 0,
			colIndex = 0;
		processedFlatList.forEach((item, index) => {
			if (item.type === 'header') {
				sectionIndex++;
				rowIndex = 0;
				colIndex = 0;
			} else if (item.type === 'item') {
				if (colIndex === 0 && newGridMap.length > 0) rowIndex++;
				newGridMap.push({ flatListIndex: index, sectionIndex, rowIndex, colIndex });
				colIndex = (colIndex + 1) % columns;
			}
		});
		return newGridMap;
	});

	const handleKeydown = (event: KeyboardEvent) => {
		if (focusManager.activeScope !== 'main-input') {
			return;
		}

		if (!['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight'].includes(event.key)) return;
		event.preventDefault();

		const currentGridIndex = gridMap.findIndex(
			(item) => item.flatListIndex === base.selectedItemIndex
		);
		if (currentGridIndex === -1) {
			if (gridMap.length > 0) base.selectedItemIndex = gridMap[0].flatListIndex;
			return;
		}

		let newGridIndex = -1;
		const currentPos = gridMap[currentGridIndex];

		if (event.key === 'ArrowLeft') {
			newGridIndex = Math.max(0, currentGridIndex - 1);
		} else if (event.key === 'ArrowRight') {
			newGridIndex = Math.min(gridMap.length - 1, currentGridIndex + 1);
		} else if (event.key === 'ArrowUp' || event.key === 'ArrowDown') {
			const direction = event.key === 'ArrowDown' ? 1 : -1;
			const targetRowIndex = currentPos.rowIndex + direction;
			const itemsInSameSection = gridMap.filter(
				(item) => item.sectionIndex === currentPos.sectionIndex
			);
			let itemsInTargetRow = itemsInSameSection.filter((item) => item.rowIndex === targetRowIndex);

			if (itemsInTargetRow.length === 0) {
				const targetSectionIndex = currentPos.sectionIndex + direction;
				const itemsInTargetSection = gridMap.filter(
					(item) => item.sectionIndex === targetSectionIndex
				);
				if (itemsInTargetSection.length > 0) {
					const rows = [...new Set(itemsInTargetSection.map((i) => i.rowIndex))].sort(
						(a, b) => a - b
					);
					itemsInTargetRow = itemsInTargetSection.filter(
						(i) => i.rowIndex === (direction === 1 ? rows[0] : rows.at(-1))
					);
				}
			}

			if (itemsInTargetRow.length > 0) {
				const targetItem =
					itemsInTargetRow.find((item) => item.colIndex === currentPos.colIndex) ??
					itemsInTargetRow.at(-1)!;
				newGridIndex = gridMap.indexOf(targetItem);
			}
		}

		if (newGridIndex !== -1) {
			base.selectedItemIndex = gridMap[newGridIndex].flatListIndex;
		}
	};

	return {
		get flatList() {
			return processedFlatList;
		},
		get virtualListItems() {
			return virtualListItems;
		},
		get selectedItemIndex() {
			return base.selectedItemIndex;
		},
		setSelectedItemIndex: (index: number) => {
			base.selectedItemIndex = index;
		},
		set vlistInstance(instance: VListHandle | undefined) {
			vlistInstance = instance;
		},
		handleKeydown
	};
}
