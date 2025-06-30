<script lang="ts">
	import type { Snippet } from 'svelte';
	import { actionBus } from '$lib/actions.svelte';

	type Props = {
		header: Snippet;
		content: Snippet;
		footer: Snippet;
	};
	let { header, content, footer }: Props = $props();

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Enter') {
			if (
				event.target instanceof HTMLElement &&
				event.target.closest('[data-slot="dropdown-menu-content"]')
			) {
				console.log('Dropdown menu content');
				return;
			}
			event.preventDefault();

			if (event.ctrlKey && !event.metaKey && !event.shiftKey) {
				actionBus.executeSecondary();
			} else if (!event.metaKey && !event.ctrlKey && !event.shiftKey) {
				actionBus.executePrimary();
			}
		}
	}
</script>

<svelte:window onkeydown={handleKeydown} />

<main class="bg-background text-foreground flex h-screen flex-col">
	{@render header()}
	{@render content()}
	{@render footer()}
</main>
