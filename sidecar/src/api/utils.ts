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

export const createSlottedComponent = (baseName: string, accessoryPropNames: string[]) => {
	const _AccessorySlot = createAccessorySlot();
	const Primitive = createWrapperComponent(baseName);
	const SlottedComponent = (props: { [key: string]: any; children?: React.ReactNode }) => {
		const { children, ...rest } = props;
		const accessoryElements = [];
		for (const name of accessoryPropNames) {
			if (rest[name]) {
				accessoryElements.push(jsx(_AccessorySlot, { name, children: rest[name] }));
				delete rest[name];
			}
		}
		return jsx(Primitive, { ...rest, children: [children, ...accessoryElements].filter(Boolean) });
	};
	SlottedComponent.displayName = baseName;
	return SlottedComponent;
};
