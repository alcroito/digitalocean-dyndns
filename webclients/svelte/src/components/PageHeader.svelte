<script>
	import NavBar from './NavBar.svelte';
</script>

<div class="header-wrapper">
	<header class="header container">
		<NavBar />
	</header>
</div>

<style lang="scss">
	@use '../variables.scss';

	.header {
		max-width: 760px;
	}

	.header-wrapper {
		border-bottom: 1px solid variables.$page-header-border-color;
		background-color: #fff;
	}
</style>
