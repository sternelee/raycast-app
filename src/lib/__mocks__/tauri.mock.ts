import { vi, type Mock, type MockedObject } from 'vitest';
import * as core from '@tauri-apps/api/core';
import * as os from '@tauri-apps/plugin-os';
import * as clipboard from '@tauri-apps/plugin-clipboard-manager';

type Promisable<T> = T | Promise<T>;
type MockFactoryWithHelper<M = unknown> = (
	importOriginal: <T extends M = M>() => Promise<T>
) => Promisable<Partial<M>>;

const createApiMock: MockFactoryWithHelper = vi.hoisted(() => async (importOriginal) => {
	const module = (await importOriginal()) as object;

	const mocks: Record<string, Mock> = {};
	for (const [key, value] of Object.entries(module)) {
		if (typeof value === 'function') {
			mocks[key] = vi.fn();
		}
	}

	const mock = { ...module, ...mocks };
	return { ...mock, default: mock };
});

vi.mock('@tauri-apps/api/core', createApiMock);
vi.mock('@tauri-apps/plugin-os', createApiMock);
vi.mock('@tauri-apps/plugin-clipboard-manager', createApiMock);

export const mockedCore = core as MockedObject<typeof core>;
export const mockedOs = os as MockedObject<typeof os>;
export const mockedClipboard = clipboard as MockedObject<typeof clipboard>;
