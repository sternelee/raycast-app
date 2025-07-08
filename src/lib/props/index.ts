import { z } from 'zod/v4';
import type { UINode } from '../types';
import {
	ActionPropsSchema,
	ActionPushPropsSchema,
	ActionPanelPropsSchema,
	ActionPanelSectionPropsSchema,
	ActionCopyToClipboardPropsSchema,
	ActionOpenInBrowserPropsSchema,
	ActionSubmitFormPropsSchema
} from './actions';
import {
	DetailPropsSchema,
	DetailMetadataPropsSchema,
	DetailMetadataLabelPropsSchema,
	DetailMetadataLinkPropsSchema,
	DetailMetadataTagListPropsSchema,
	DetailMetadataTagListItemPropsSchema,
	DetailMetadataSeparatorPropsSchema
} from './detail';
import {
	GridPropsSchema,
	GridItemPropsSchema,
	GridSectionPropsSchema,
	GridEmptyViewPropsSchema
} from './grid';
import { ViewSectionPropsSchema } from './section';
import {
	FormPropsSchema,
	FormTextFieldPropsSchema,
	FormTextAreaPropsSchema,
	FormDescriptionPropsSchema,
	FormLinkAccessoryPropsSchema
} from './form';
import {
	ListPropsSchema,
	ListItemPropsSchema,
	ListItemDetailPropsSchema,
	ListItemDetailMetadataPropsSchema
} from './list';
import {
	DropdownPropsSchema,
	DropdownSectionPropsSchema,
	DropdownItemPropsSchema
} from './dropdown';

export * from './actions';
export * from './detail';
export * from './grid';
export * from './list';
export * from './form';
export * from '@raycast-linux/protocol';
export * from './color';
export * from './dropdown';
export * from './section';

export const componentSchemas = {
	Action: ActionPropsSchema,
	'Action.Push': ActionPushPropsSchema,
	ActionPanel: ActionPanelPropsSchema,
	'ActionPanel.Section': ActionPanelSectionPropsSchema,
	'Action.CopyToClipboard': ActionCopyToClipboardPropsSchema,
	'Action.OpenInBrowser': ActionOpenInBrowserPropsSchema,
	'Action.SubmitForm': ActionSubmitFormPropsSchema,

	List: ListPropsSchema,
	'List.Section': ViewSectionPropsSchema,
	'List.Item': ListItemPropsSchema,
	'List.Dropdown': DropdownPropsSchema,
	'List.Dropdown.Section': DropdownSectionPropsSchema,
	'List.Dropdown.Item': DropdownItemPropsSchema,
	'List.Item.Detail': ListItemDetailPropsSchema,
	'List.Item.Detail.Metadata': ListItemDetailMetadataPropsSchema,
	'List.Item.Detail.Metadata.Label': DetailMetadataLabelPropsSchema,
	'List.Item.Detail.Metadata.Link': DetailMetadataLinkPropsSchema,
	'List.Item.Detail.Metadata.TagList': DetailMetadataTagListPropsSchema,
	'List.Item.Detail.Metadata.TagList.Item': DetailMetadataTagListItemPropsSchema,
	'List.Item.Detail.Metadata.Separator': DetailMetadataSeparatorPropsSchema,

	Grid: GridPropsSchema,
	'Grid.Section': GridSectionPropsSchema,
	'Grid.Item': GridItemPropsSchema,
	'Grid.EmptyView': GridEmptyViewPropsSchema,
	'Grid.Dropdown': DropdownPropsSchema,
	'Grid.Dropdown.Section': DropdownSectionPropsSchema,
	'Grid.Dropdown.Item': DropdownItemPropsSchema,

	Form: FormPropsSchema,
	'Form.TextField': FormTextFieldPropsSchema,
	'Form.TextArea': FormTextAreaPropsSchema,
	'Form.Description': FormDescriptionPropsSchema,
	'Form.Dropdown': DropdownPropsSchema,
	'Form.Dropdown.Item': DropdownItemPropsSchema,
	'Form.Dropdown.Section': DropdownSectionPropsSchema,
	'Form.LinkAccessory': FormLinkAccessoryPropsSchema,

	Detail: DetailPropsSchema,
	'Detail.Metadata': DetailMetadataPropsSchema,
	'Detail.Metadata.Label': DetailMetadataLabelPropsSchema,
	'Detail.Metadata.Link': DetailMetadataLinkPropsSchema,
	'Detail.Metadata.TagList': DetailMetadataTagListPropsSchema,
	'Detail.Metadata.TagList.Item': DetailMetadataTagListItemPropsSchema,
	'Detail.Metadata.Separator': DetailMetadataSeparatorPropsSchema
};

export type Schemas = typeof componentSchemas;
export type ComponentType = keyof Schemas;

export function getTypedProps<T extends ComponentType>(
	node: UINode & { type: T }
): z.infer<Schemas[T]> | null {
	const result = (componentSchemas[node.type] as z.ZodTypeAny).safeParse(node.props);
	if (!result.success) {
		console.error(
			`[Props Validation Error] For node ${node.id} (type: ${node.type}):`,
			z.prettifyError(result.error)
		);
		return null;
	}
	return result.data as z.infer<Schemas[T]>;
}
