import { z } from 'zod/v4';

export const GridInsetSchema = z.enum(['small', 'medium', 'large']);
export type GridInset = z.infer<typeof GridInsetSchema>;

export const GridPropsSchema = z.object({
	filtering: z.boolean().optional(),
	throttle: z.boolean().default(false),
	columns: z.number().default(6),
	searchBarPlaceholder: z.string().optional(),
	onSearchTextChange: z.boolean().optional(),
	isLoading: z.boolean().default(false),
	inset: GridInsetSchema.optional()
});
export type GridProps = z.infer<typeof GridPropsSchema>;

export const GridItemPropsSchema = z.object({
	content: z.string(),
	title: z.string().optional(),
	subtitle: z.string().optional(),
	keywords: z.array(z.string())
});
export type GridItemProps = z.infer<typeof GridItemPropsSchema>;
