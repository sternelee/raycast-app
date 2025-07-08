<script lang="ts">
	import type { GridItemProps, GridInset, GridFit } from '$lib/props';
	import type { HTMLButtonAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils';
	import Icon from '../Icon.svelte';

	type Props = {
		props: GridItemProps;
		selected: boolean;
		inset?: GridInset;
		fit?: GridFit;
		aspectRatio?: string;
	} & HTMLButtonAttributes;

	let { props, selected, inset, fit, aspectRatio, ...restProps }: Props = $props();

	const paddingClass = $derived.by(() => {
		switch (inset) {
			case 'small':
				return 'p-1.5';
			case 'medium':
				return 'p-2.5';
			case 'large':
				return 'p-4';
			default:
				return 'p-1';
		}
	});

	const content = $derived(
		typeof props.content === 'object' && 'value' in props.content
			? props.content.value
			: props.content
	);
	const tooltip = $derived(
		typeof props.content === 'object' && 'tooltip' in props.content
			? props.content.tooltip
			: undefined
	);
</script>

<button
	type="button"
	class={cn('flex w-full flex-col text-left focus:outline-none', paddingClass)}
	{...restProps}
>
	<div
		class="hover:border-foreground/50 bg-muted mb-1 w-full overflow-hidden rounded-md border-2 {selected
			? 'border-foreground'
			: 'border-transparent'}"
		class:border-transparent={!selected}
		style:aspect-ratio={aspectRatio ?? '1'}
		title={tooltip}
	>
		<Icon icon={content} class="size-full" style="object-fit: {fit ?? 'contain'}" />
	</div>

	{#if props.title}
		<span class="truncate text-sm font-medium">{props.title}</span>
	{/if}
	{#if props.subtitle}
		<span class="text-muted-foreground truncate text-xs">{props.subtitle}</span>
	{/if}
	{#if props.accessory}
		<div
			class="text-muted-foreground mt-0.5 flex items-center gap-1 text-xs"
			title={props.accessory.tooltip}
		>
			{#if props.accessory.icon}
				<Icon icon={props.accessory.icon} class="size-3" />
			{/if}
			{#if props.accessory.text}
				<span class="truncate">{props.accessory.text}</span>
			{/if}
		</div>
	{/if}
</button>
