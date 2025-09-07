<script lang="ts">
	import { resolve } from '$app/paths';
	import { page } from '$app/stores';
	import VersionDisplay from './VersionDisplay.svelte';

	import logo from '$lib/images/logo.png';

	let burgerIsActive = false;

	// Navbar centering use in CSS https://github.com/jgthms/bulma/issues/1604
</script>

<nav class="navbar" aria-label="main navigation">
	<div class="navbar-brand ml-0">
		<a
			href="#menu"
			role="button"
			class="navbar-burger ml-0"
			aria-label="menu"
			aria-expanded="false"
			data-target="mainNavMenu"
			class:is-active={burgerIsActive}
			on:click={() => (burgerIsActive = !burgerIsActive)}
		>
			<span aria-hidden="true"></span>
			<span aria-hidden="true"></span>
			<span aria-hidden="true"></span>
		</a>

		<a class="navbar-item" href={resolve('/')}>
			<figure class="image">
				<img src={logo} alt="logo" />
			</figure>
		</a>
	</div>

	<div id="mainNavMenu" class="navbar-menu" class:is-active={burgerIsActive}>
		<div class="navbar-start is-justify-content-center is-flex-grow-1">
			<div class="navbar-item-wrapper">
				<a class="navbar-item" href={resolve('/')} class:active={$page.url.pathname == '/'}>
					<div class="icon-text">
						<span class="icon">
							<i class="fa-solid fa-house"></i>
						</span>
						<span> Dashboard </span>
					</div>
				</a>
			</div>
			<div class="navbar-item-wrapper">
				<a class="navbar-item" href="#more">
					<div class="icon-text">
						<span class="icon">
							<i class="fa-solid fa-circle-question"></i>
						</span>
						<span> More to come </span>
					</div>
				</a>
			</div>
		</div>
		<div class="navbar-end">
			<div class="navbar-item">
				<VersionDisplay />
			</div>
		</div>
	</div>
</nav>

<style lang="scss">
	@use '../../node_modules/bulma/sass/utilities/mixins.scss';

	@include mixins.from(992px) {
		.navbar a {
			font-size: 13px;
		}
	}

	@include mixins.widescreen {
		.navbar a {
			font-size: 14px;
		}
	}

	// Avoid layout shifting before image is loaded.
	.navbar-brand .image {
		width: 41px;
		height: 28px;
	}

	.navbar-menu {
		margin-right: 45px;
	}

	.navbar-item-wrapper {
		padding: 0 0.75rem;
		display: flex;
	}

	.navbar-item {
		padding: 0.5rem 0;
	}

	.navbar {
		min-height: 61px;
	}

	// Make this more like adguard.
	.navbar .navbar-start a {
		color: #9aa0ac;
		background-color: transparent;
		border-bottom: 1px solid transparent;

		&:hover {
			color: #6e7687;
			border-color: #6e7687;
			background-color: transparent;
		}
	}
	.navbar .navbar-start a.active {
		color: #5eba00;
		border-color: #5eba00;

		&:hover {
			color: #4b9400;
			border-color: #4b9400;
		}
	}
</style>
