import { z } from 'zod/v4';

const FormValueSchema = z.union([
	z.string(),
	z.number(),
	z.boolean(),
	z.array(z.string()),
	z.array(z.number()),
	z.date(),
	z.null()
]);
type FormValue = z.infer<typeof FormValueSchema>;

type FormEvent<T extends FormValue> = {
	target: {
		id: string;
		value?: T;
	};

	type: 'focus' | 'blur';
};

export const serializeEvent = (itemId: string, event: Event): FormEvent<FormValue> => {
	const value = FormValueSchema.safeParse(
		event.target && 'value' in event.target ? event.target.value : undefined
	);

	return {
		type: event.type as 'focus' | 'blur',
		target: {
			id: itemId,
			value: value.success ? value.data : undefined
		}
	};
};
