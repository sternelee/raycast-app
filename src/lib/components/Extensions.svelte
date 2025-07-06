<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { type Extension, ExtensionSchema } from '$lib/store';
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
	import ExtensionInstallConfirm from './extensions/ExtensionInstallConfirm.svelte';
	import { fetch } from '@tauri-apps/plugin-http';

	type Props = {
		onBack: () => void;
		onInstall: () => void;
	};

	type Violation = {
		commandName: string;
		reason: string;
	};

	let { onBack, onInstall }: Props = $props();

	let selectedExtension = $state<Extension | null>(null);
	let detailedExtension = $state<Extension | null>(null);
	let isDetailLoading = $state(false);
	let expandedImageUrl = $state<string | null>(null);
	let isInstalling = $state(false);
	let vlistInstance = $state<VListHandle | null>(null);
	let showConfirmationDialog = $state(false);
	let confirmationViolations = $state<Violation[]>([]);

	$effect(() => {
		const ext = viewManager.extensionToSelect;
		if (ext) {
			selectedExtension = ext;
			viewManager.extensionToSelect = null;
		}
	});

	$effect(() => {
		if (selectedExtension && selectedExtension.id !== detailedExtension?.id) {
			detailedExtension = null;
			isDetailLoading = true;
			const fetchDetails = async () => {
				try {
					const res = await fetch(
						`https://backend.raycast.com/api/v1/extensions/${selectedExtension!.author.handle}/${selectedExtension!.name}`
					);
					if (!res.ok) throw new Error(`Failed to fetch extension details: ${res.status}`);
					const json = await res.json();
					const parsed = ExtensionSchema.parse(json);
					detailedExtension = parsed;
				} catch (e) {
					console.error('Failed to fetch or parse extension details, using list data.', e);
					detailedExtension = selectedExtension;
				} finally {
					isDetailLoading = false;
				}
			};
			fetchDetails();
		} else if (!selectedExtension) {
			detailedExtension = null;
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
		const extensionToInstall = detailedExtension || selectedExtension;
		if (!extensionToInstall || isInstalling) return;
		isInstalling = true;
		try {
			const result = await invoke<{
				status: 'success' | 'requiresConfirmation';
				violations?: Violation[];
			}>('install_extension', {
				downloadUrl: extensionToInstall.download_url,
				slug: extensionToInstall.name,
				force: false
			});

			if (result.status === 'success') {
				onInstall();
			} else if (result.status === 'requiresConfirmation' && result.violations) {
				confirmationViolations = result.violations;
				showConfirmationDialog = true;
			}
		} catch (e) {
			console.error('Installation failed', e);
		} finally {
			isInstalling = false;
		}
	}

	async function handleForceInstall() {
		showConfirmationDialog = false;
		const extensionToInstall = detailedExtension || selectedExtension;
		if (!extensionToInstall) return;
		isInstalling = true;
		try {
			await invoke('install_extension', {
				downloadUrl: extensionToInstall.download_url,
				slug: extensionToInstall.name,
				force: true
			});
			onInstall();
		} catch (e) {
			console.error('Forced installation failed', e);
		} finally {
			isInstalling = false;
		}
	}
</script>

<svelte:window onkeydown={handleGlobalKeyDown} />

<main class="bg-background text-foreground flex h-screen flex-col">
	<header class="relative flex h-15 shrink-0 items-center pr-4 pl-[18px]">
		<Button
			size="icon"
			onclick={() => (selectedExtension ? (selectedExtension = null) : onBack())}
			variant="secondary"
			class="size-6"
		>
			<Icon icon="arrow-left-16" />
		</Button>
		{#if !selectedExtension}
			<HeaderInput
				placeholder="Search Store for extensions..."
				bind:value={extensionsStore.searchText}
				autofocus
			/>
			<CategoryFilter />
		{/if}
		<LoadingIndicator
			isLoading={(extensionsStore.isLoading && !selectedExtension) || isDetailLoading}
		/>
	</header>

	{#if selectedExtension}
		{@const extensionToShow = detailedExtension || selectedExtension}
		<ExtensionDetailView
			extension={extensionToShow}
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

<ExtensionInstallConfirm
	bind:open={showConfirmationDialog}
	violations={confirmationViolations}
	onconfirm={handleForceInstall}
	oncancel={() => (showConfirmationDialog = false)}
/>
