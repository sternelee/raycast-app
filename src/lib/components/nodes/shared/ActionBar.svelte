<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { ImageLike } from '@raycast-linux/protocol';
	import Icon from '$lib/components/Icon.svelte';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import type { ButtonProps } from '$lib/components/ui/button';

	type Props = {
		title?: string | Snippet;
		icon?: ImageLike | null;
		primaryAction?: Snippet<[{ props: ButtonProps }]>;
		actions?: Snippet;
	};

	let { title, icon, primaryAction, actions }: Props = $props();
</script>

<footer class="bg-card flex h-10 shrink-0 items-center border-t px-2">
	{#if title || icon}
		<div class="flex min-w-0 items-center gap-2">
			{#if icon}
				<Icon {icon} class="size-5 shrink-0" />
			{/if}
			{#if title}
				{#if typeof title === 'string'}
					<span class="text-muted-foreground truncate text-sm">{title}</span>
				{:else}
					{@render title()}
				{/if}
			{/if}
		</div>
	{/if}

	<div class="ml-auto flex items-center">
		{#if primaryAction}
			<div class="peer order-1">
				{@render primaryAction({ props: { variant: 'ghost', size: 'action' } })}
			</div>
		{/if}
		{#if actions}
			<div class="peer order-3">
				{@render actions()}
			</div>
		{/if}
		{#if actions}
			{#if primaryAction}
				<Separator
					orientation="vertical"
					class="order-2 mr-1 ml-2 !h-3 !w-[2px] transition-opacity peer-hover:opacity-0"
				/>
			{/if}
		{/if}
	</div>
</footer>
