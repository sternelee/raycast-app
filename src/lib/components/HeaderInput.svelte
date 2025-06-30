<script lang="ts">
	import { Input, type InputProps } from '$lib/components/ui/input';
	import { cn } from '$lib/utils';

	let { value = $bindable(), ref = $bindable(), class: className, ...rest }: InputProps = $props();
</script>

<Input
	type="text"
	class={cn(
		'h-auto w-full rounded-none border-none !bg-transparent pr-0 text-base shadow-none focus-visible:ring-0 focus-visible:ring-offset-0',
		className
	)}
	bind:ref
	bind:value
	{...rest}
/>
