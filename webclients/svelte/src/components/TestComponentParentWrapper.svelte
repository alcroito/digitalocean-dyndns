<script lang="ts">
	export let childComponent: any;
	export let parentComponent: any;
</script>

<svelte:component this={parentComponent}> 
	<svelte:component this={childComponent} />
</svelte:component>
