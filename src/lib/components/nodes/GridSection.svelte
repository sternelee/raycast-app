<script lang="ts">
	import type { ViewSectionProps } from '$lib/props';

	type Props = {
		props: ViewSectionProps;
	};

	let { props }: Props = $props();
</script>

<h3 class="col-start-1 -col-end-1 -mb-1 pt-2.5 text-xs font-semibold text-gray-500 uppercase">
	{props.title}
</h3>
