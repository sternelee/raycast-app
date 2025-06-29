import { z } from 'zod/v4';
import { GridInsetSchema } from './grid';

export const ViewSectionPropsSchema = z.object({
	title: z.string().optional(),
	inset: GridInsetSchema.optional()
});
export type ViewSectionProps = z.infer<typeof ViewSectionPropsSchema>;
