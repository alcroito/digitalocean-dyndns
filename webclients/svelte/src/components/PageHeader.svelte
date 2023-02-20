<script>
	import NavBar from './NavBar.svelte';
</script>

<div class="page-header-wrapper">
	<header class="page-header container is-max-desktop">
		<NavBar />
	</header>
</div>

<style lang="scss">
	@import '../variables.scss';

	.page-header {
		max-width: 760px;
	}

	.page-header-wrapper {
		border-bottom: 1px solid $page-header-border-color;
		background-color: #fff;
	}
</style>
