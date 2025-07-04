<script lang="ts">
	import type { Datum } from '$lib/store';
	import { Button } from '$lib/components/ui/button';
	import { ArrowLeft } from '@lucide/svelte';
	import Icon from './Icon.svelte';
	import { invoke } from '@tauri-apps/api/core';
	import ExtensionListView from './extensions/ExtensionListView.svelte';
	import ExtensionDetailView from './extensions/ExtensionDetailView.svelte';
	import ImageLightbox from './extensions/ImageLightbox.svelte';
	import CategoryFilter from './extensions/CategoryFilter.svelte';
	import { extensionsStore } from './extensions/store.svelte';
	import LoadingIndicator from './LoadingIndicator.svelte';
	import type { VListHandle } from 'virtua/svelte';
	import HeaderInput from './HeaderInput.svelte';
	import { viewManager } from '$lib/viewManager.svelte';

	type Props = {
		onBack: () => void;
		onInstall: () => void;
	};

	let { onBack, onInstall }: Props = $props();

	let selectedExtension = $state<Datum | null>(null);
	let expandedImageUrl = $state<string | null>(null);
	let isInstalling = $state(false);
	let vlistInstance = $state<VListHandle | null>(null);

	$effect(() => {
		const ext = viewManager.extensionToSelect;
		if (ext) {
			selectedExtension = ext;
			viewManager.extensionToSelect = null;
		}
	});

	const handleScroll = () => {
		if (!vlistInstance) return;
		if (
			vlistInstance.getScrollSize() -
				vlistInstance.getScrollOffset() -
				vlistInstance.getViewportSize() <
			500
		) {
			extensionsStore.loadMore();
		}
	};

	function handleGlobalKeyDown(e: KeyboardEvent) {
		if (e.key === 'Escape' && !e.defaultPrevented) {
			e.preventDefault();
			if (expandedImageUrl) {
				expandedImageUrl = null;
			} else if (selectedExtension) {
				selectedExtension = null;
			} else {
				onBack();
			}
			return;
		}
	}

	async function handleInstall() {
		if (!selectedExtension || isInstalling) return;
		isInstalling = true;
		try {
			await invoke('install_extension', {
				downloadUrl: selectedExtension.download_url,
				slug: selectedExtension.name
			});
			onInstall();
		} catch (e) {
			console.error('Installation failed', e);
		} finally {
			isInstalling = false;
		}
	}
</script>

<svelte:window onkeydown={handleGlobalKeyDown} />

<main class="bg-background text-foreground flex h-screen flex-col">
	<header class="relative mb-2 flex h-15 shrink-0 items-center border-b">
		<Button
			variant="ghost"
			size="icon"
			onclick={() => (selectedExtension ? (selectedExtension = null) : onBack())}
		>
			<ArrowLeft class="size-5" />
		</Button>
		{#if selectedExtension}
			<div class="flex items-center gap-3 px-2">
				<Icon
					icon={selectedExtension.icons.light
						? { source: selectedExtension.icons.light, mask: 'circle' }
						: undefined}
					class="size-6"
				/>
				<h1 class="text-lg font-medium">{selectedExtension.title}</h1>
			</div>
		{:else}
			<HeaderInput
				placeholder="Search Store for extensions..."
				bind:value={extensionsStore.searchText}
				autofocus
			/>
			<CategoryFilter />
		{/if}
		<LoadingIndicator isLoading={extensionsStore.isLoading && !selectedExtension} />
	</header>

	{#if selectedExtension}
		<ExtensionDetailView
			extension={selectedExtension}
			{isInstalling}
			onInstall={handleInstall}
			onOpenLightbox={(imageUrl) => (expandedImageUrl = imageUrl)}
		/>
	{:else}
		<div class="grow overflow-y-auto" role="listbox" tabindex={-1}>
			<ExtensionListView
				onSelect={(ext) => (selectedExtension = ext)}
				onScroll={handleScroll}
				bind:vlistInstance
			/>
		</div>
	{/if}
</main>

{#if expandedImageUrl}
	<ImageLightbox imageUrl={expandedImageUrl} onClose={() => (expandedImageUrl = null)} />
{/if}
