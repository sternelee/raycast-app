import { mockedCore, mockedClipboard } from '$lib/__mocks__/tauri.mock';
import { render, screen, cleanup, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import Extensions from './Extensions.svelte';
import type { Extension } from '$lib/store';
import { openUrl } from '@tauri-apps/plugin-opener';

if (typeof window !== 'undefined') {
	window.HTMLElement.prototype.animate = () => ({
		finished: Promise.resolve({} as Animation),
		cancel: () => {},
		play: () => {},
		pause: () => {},
		reverse: () => {},
		finish: () => {},
		commitStyles: () => {},
		updatePlaybackRate: () => {},
		persist: () => {},
		startTime: 0,
		currentTime: 0,
		timeline: null,
		playbackRate: 1,
		pending: false,
		playState: 'idle',
		ready: Promise.resolve({} as Animation),
		onfinish: null,
		oncancel: null,
		onremove: null,
		effect: null,
		id: '',
		replaceState: 'active',
		addEventListener: () => {},
		removeEventListener: () => {},
		dispatchEvent: () => false
	});
}

vi.mock('@tauri-apps/plugin-opener', () => ({
	openUrl: vi.fn()
}));

const mockedFetch = vi.fn();
vi.mock('@tauri-apps/plugin-http', () => ({
	fetch: (url: string) => mockedFetch(url)
}));

const extensionsStore = vi.hoisted(() => ({
	searchText: '',
	searchResults: [] as Extension[],
	selectedCategory: 'All Categories',
	extensions: [] as Extension[],
	featuredExtensions: [] as Extension[],
	trendingExtensions: [] as Extension[],
	isSearching: false,
	isLoading: false,
	selectedIndex: 0,
	loadMore: vi.fn(),
	_reset: function () {
		this.searchText = '';
		this.searchResults = [];
		this.selectedCategory = 'All Categories';
		this.extensions = [];
		this.featuredExtensions = [];
		this.trendingExtensions = [];
		this.isSearching = false;
		this.isLoading = false;
		this.selectedIndex = 0;
		this.loadMore.mockClear();
	}
}));
vi.mock('./extensions/store.svelte', () => ({
	extensionsStore
}));

const viewManager = vi.hoisted(() => ({
	extensionToSelect: null,
	showSettings: vi.fn(),
	_reset: function () {
		this.extensionToSelect = null;
		this.showSettings.mockClear();
	}
}));
vi.mock('$lib/viewManager.svelte', () => ({
	viewManager
}));

const mockAuthor = {
	name: 'Raycast',
	handle: 'raycast',
	avatar: 'https://raycast.com/avatar.png',
	initials: 'RC',
	avatar_placeholder_color: '#A067DC' as const
};

const createMockExtension = (
	id: string,
	name: string,
	category: string,
	options: Partial<Extension> = {}
): Extension => ({
	id,
	name,
	native_id: null,
	title: name,
	description: `Description for ${name}`,
	author: mockAuthor,
	owner: mockAuthor,
	icons: { light: 'icon.png', dark: 'icon.png' },
	categories: [category],
	store_url: `https://raycast.com/raycast/${name}`,
	download_url: `https://raycast.com/api/v1/extensions/raycast/${name}/download`,
	readme_url: `https://github.com/raycast/extensions/blob/main/extensions/${name}/README.md`,
	source_url: `https://github.com/raycast/extensions/tree/main/extensions/${name}`,
	seo_categories: [],
	platforms: null,
	created_at: Date.now(),
	kill_listed_at: null,
	status: 'active',
	is_new: false,
	access: 'public',
	download_count: 100,
	commit_sha: '12345',
	relative_path: `extensions/${name}`,
	api_version: '1.0',
	prompt_examples: [],
	metadata_count: 0,
	updated_at: Date.now(),
	readme_assets_path: '',
	commands: [],
	tools: [],
	contributors: [],
	...options
});

const mockFeatured = createMockExtension('1', 'Featured Extension', 'Featured');
const mockTrending = createMockExtension('2', 'Trending Extension', 'Trending');
const mockRegular = createMockExtension('3', 'Regular Extension', 'Productivity');
const mockDetailed = createMockExtension('1', 'Featured Extension', 'Featured');

describe('Extensions.svelte', () => {
	const onBack = vi.fn();
	const onInstall = vi.fn();
	const user = userEvent.setup();

	beforeEach(() => {
		cleanup();
		vi.clearAllMocks();
		extensionsStore._reset();
		viewManager._reset();
		mockedFetch.mockClear();
		mockedCore.invoke.mockResolvedValue({ status: 'success' });
	});

	describe('1. Initial Rendering and State', () => {
		it('should show a loading indicator while loading', () => {
			extensionsStore.isLoading = true;
			render(Extensions, { onBack, onInstall });
			expect(screen.getByTestId('loading-indicator')).toBeInTheDocument();
		});

		it('should render the header and search input', () => {
			render(Extensions, { onBack, onInstall });
			expect(screen.getByPlaceholderText('Search Store for extensions...')).toBeInTheDocument();
		});

		it('should display extensions with headers when loaded', async () => {
			extensionsStore.featuredExtensions = [mockFeatured];
			extensionsStore.trendingExtensions = [mockTrending];
			extensionsStore.extensions = [mockRegular];
			render(Extensions, { onBack, onInstall });

			expect(await screen.findByText('Featured')).toBeInTheDocument();
			expect(await screen.findByText(mockFeatured.title)).toBeInTheDocument();

			expect(await screen.findByText('Trending')).toBeInTheDocument();
			expect(await screen.findByText(mockTrending.title)).toBeInTheDocument();
		});
	});

	describe('2. Search and Filtering', () => {
		it('should update searchText in the store when typing in search input', async () => {
			render(Extensions, { onBack, onInstall });
			const searchInput = screen.getByPlaceholderText('Search Store for extensions...');
			await user.type(searchInput, 'test search');
			expect(extensionsStore.searchText).toBe('test search');
		});

		it('should display only search results when searchText is present', async () => {
			extensionsStore.searchText = 'search';
			extensionsStore.searchResults = [mockRegular];
			render(Extensions, { onBack, onInstall });

			expect(await screen.findByText('Search Results')).toBeInTheDocument();
			expect(screen.getByText(mockRegular.title)).toBeInTheDocument();
			expect(screen.queryByText('Featured')).not.toBeInTheDocument();
		});

		it('should display filtered extensions when a category is selected', async () => {
			extensionsStore.selectedCategory = 'Productivity';
			extensionsStore.extensions = [mockFeatured, mockRegular];
			render(Extensions, { onBack, onInstall });

			await waitFor(() => {
				expect(screen.getByText('Productivity', { selector: 'button' })).toBeInTheDocument();
			});
			expect(screen.getByText(mockRegular.title)).toBeInTheDocument();
			expect(screen.queryByText(mockFeatured.title)).not.toBeInTheDocument();
		});
	});

	describe('3. Extension Selection and Detail View', () => {
		beforeEach(() => {
			extensionsStore.featuredExtensions = [mockFeatured];
			mockedFetch.mockResolvedValue({
				ok: true,
				json: () => Promise.resolve(mockDetailed)
			});
		});

		it('should switch to detail view on item click', async () => {
			render(Extensions, { onBack, onInstall });
			const extensionItem = await screen.findByText(mockFeatured.title);
			await user.click(extensionItem);

			expect(await screen.findByText('Install Extension')).toBeInTheDocument();
			expect(screen.queryByRole('listbox')).not.toBeInTheDocument();
		});

		it('should fetch detailed extension data when an item is selected', async () => {
			render(Extensions, { onBack, onInstall });
			const extensionItem = await screen.findByText(mockFeatured.title);
			await user.click(extensionItem);

			await waitFor(() => {
				expect(mockedFetch).toHaveBeenCalledWith(
					`https://backend.raycast.com/api/v1/extensions/${mockFeatured.author.handle}/${mockFeatured.name}`
				);
			});
		});
	});

	describe('4. Actions and Shortcuts', () => {
		beforeEach(() => {
			extensionsStore.extensions = [
				mockRegular,
				createMockExtension('4', 'no-readme', 'Tools', { readme_url: null })
			];
			extensionsStore.selectedIndex = 1;
		});

		it('should open extension in browser via action menu', async () => {
			render(Extensions, { onBack, onInstall });
			const menuTrigger = await screen.findByTestId('action-menu-trigger');
			await user.click(menuTrigger);

			const openButton = await screen.findByText('Open in Browser');
			await user.click(openButton);
			expect(openUrl).toHaveBeenCalledWith(mockRegular.store_url);
		});

		it('should copy extension URL via action menu', async () => {
			render(Extensions, { onBack, onInstall });
			const menuTrigger = await screen.findByTestId('action-menu-trigger');
			await user.click(menuTrigger);

			const copyButton = await screen.findByText('Copy Extension URL');
			await user.click(copyButton);
			expect(mockedClipboard.writeText).toHaveBeenCalledWith(mockRegular.store_url);
		});

		it('should view source code via action menu', async () => {
			render(Extensions, { onBack, onInstall });
			const menuTrigger = await screen.findByTestId('action-menu-trigger');
			await user.click(menuTrigger);

			const sourceButton = await screen.findByText('View Source Code');
			await user.click(sourceButton);
			expect(openUrl).toHaveBeenCalledWith(mockRegular.source_url);
		});

		it('should disable README button if no readme_url exists', async () => {
			extensionsStore.selectedIndex = 2;
			render(Extensions, { onBack, onInstall });
			const menuTrigger = await screen.findByTestId('action-menu-trigger');
			await waitFor(() => user.click(menuTrigger));

			const readmeButton = await screen.findByText('View README');
			expect(readmeButton).toHaveAttribute('aria-disabled', 'true');
		});
	});
});
