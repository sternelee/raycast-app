<script lang="ts">
	import type { GridItemProps } from '$lib/props';
	import type { HTMLButtonAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils';
	import type { GridInset } from '$lib/props/grid';
	import Icon from '../Icon.svelte';

	type Props = {
		props: GridItemProps;
		selected: boolean;
		inset?: GridInset;
	} & HTMLButtonAttributes;

	let { props, selected, inset, ...restProps }: Props = $props();

	const paddingClass = $derived(() => {
		switch (inset) {
			case 'small':
				return 'p-1';
			case 'medium':
				return 'p-2';
			case 'large':
				return 'p-4';
			default:
				return 'px-4 py-2';
		}
	});
</script>

<button type="button" class={cn('flex w-full flex-col text-left', paddingClass)} {...restProps}>
	<div
		class="hover:border-foreground/50 bg-muted mb-1 aspect-square w-full rounded-md border-2 border-transparent"
		class:!border-foreground={selected}
	>
		<Icon icon={props.content} class="size-full" />
	</div>

	{#if props.title}
		<span class="text-sm font-medium">{props.title}</span>
	{/if}
	{#if props.subtitle}
		<span class="text-muted-foreground text-sm">{props.subtitle}</span>
	{/if}
</button>
