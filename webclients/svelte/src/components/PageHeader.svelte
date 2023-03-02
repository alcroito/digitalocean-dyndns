<script>
	import NavBar from './NavBar.svelte';
</script>

<div class="header-wrapper">
	<header class="header container">
		<NavBar />
	</header>
</div>

<style lang="scss">
	@import '../variables.scss';

	.header {
		max-width: 760px;
	}

	.header-wrapper {
		border-bottom: 1px solid $page-header-border-color;
		background-color: #fff;
	}
</style>
