import type {
	ActionCopyToClipboardProps,
	ActionOpenInBrowserProps,
	ActionProps,
	componentSchemas,
	ComponentType,
	Schemas
} from '$lib/props';
import type { UINode } from '$lib/types';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { openUrl } from '@tauri-apps/plugin-opener';
import z from 'zod/v4';

export type ActionDefinition = ActionProps & { handler?: () => void; disabled?: boolean };

// TODO: naming?
export const nodeToActionDefinition = (
	node: UINode,
	onDispatch: (instanceId: number, handlerName: string, args: unknown[]) => void
): ActionDefinition => {
	switch (node.type) {
		case 'Action.CopyToClipboard': {
			const copyProps = node.props as ActionCopyToClipboardProps;

			return {
				title: copyProps.title ?? 'Copy to Clipboard',
				handler: () => {
					writeText(copyProps.content);
					onDispatch(node.id, 'onCopy', []);
				}
			};
		}
		case 'Action.OpenInBrowser': {
			const openProps = node.props as ActionOpenInBrowserProps;

			return {
				title: 'Open in Browser',
				handler: () => {
					openUrl(openProps.url);
					onDispatch(node.id, 'onOpenInBrowser', []);
				}
			};
		}
		case 'Action.SubmitForm': {
			return {
				title: node.props.title ?? 'Submit Form',
				handler: () => onDispatch(node.id, 'onSubmit', [])
			};
		}
		case 'Action.Push':
		case 'Action':
		default: {
			return {
				title: node.props.title,
				handler: () => onDispatch(node.id, 'onAction', [])
			};
		}
	}
};
