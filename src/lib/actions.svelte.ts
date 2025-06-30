type ActionFunction = (() => void) | null;

let primaryAction: ActionFunction = null;
let secondaryAction: ActionFunction = null;

const createActionBus = () => {
	return {
		registerPrimary: (fn: () => void) => {
			primaryAction = fn;
		},
		unregisterPrimary: () => {
			primaryAction = null;
		},
		registerSecondary: (fn: () => void) => {
			secondaryAction = fn;
		},
		unregisterSecondary: () => {
			secondaryAction = null;
		},
		executePrimary: () => {
			primaryAction?.();
		},
		executeSecondary: () => {
			secondaryAction?.();
		}
	};
};

export const actionBus = createActionBus();
