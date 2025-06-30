import React, { type ElementType } from 'react';
import { jsx } from 'react/jsx-runtime';

export const createLocalStorage = () => {
	const storage = new Map<string, string>();
	return {
		getItem: async (key: string) => storage.get(key),
		setItem: async (key: string, value: string) => storage.set(key, value),
		removeItem: async (key: string) => storage.delete(key),
		clear: async () => storage.clear()
	};
};

export const createWrapperComponent = (name: string) => {
	const Component = React.forwardRef(
		({ children, ...rest }: { children?: React.ReactNode }, ref) => {
			return jsx(name as ElementType, { ...rest, children, ref });
		}
	);
	Component.displayName = name;
	return Component;
};

export const createAccessorySlot = () => createWrapperComponent('_AccessorySlot');
