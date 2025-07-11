import type { ActionCopyToClipboardProps, ActionOpenInBrowserProps, ActionProps } from '$lib/props';
import type { UINode } from '$lib/types';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { openUrl } from '@tauri-apps/plugin-opener';

export type ActionDefinition = ActionProps & { handler?: () => void; disabled?: boolean };

// TODO: naming?
export const nodeToActionDefinition = (
	node: UINode,
	onDispatch: (instanceId: number, handlerName: string, args: unknown[]) => void
): ActionDefinition => {
	const title = typeof node.props.title === 'string' ? node.props.title : undefined;

	switch (node.type) {
		case 'Action.CopyToClipboard': {
			const copyProps = node.props as ActionCopyToClipboardProps;

			return {
				title: title ?? 'Copy to Clipboard',
				...copyProps,
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
				...openProps,
				handler: () => {
					openUrl(openProps.url);
					onDispatch(node.id, 'onOpenInBrowser', []);
				}
			};
		}
		case 'Action.SubmitForm': {
			return {
				title: title ?? 'Submit Form',
				...node.props,
				handler: () => onDispatch(node.id, 'onSubmit', [])
			};
		}
		case 'Action.Push':
		case 'Action':
		default: {
			return {
				title: title!,
				...node.props,
				handler: () => onDispatch(node.id, 'onAction', [])
			};
		}
	}
};
