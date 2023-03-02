<script>
	import DomainRecordIpChanges from '../components/DomainRecordIpChanges.svelte';
</script>

<div class="container px-3-mobile">
	<div class="py-5 has-text-centered">
		<h1 class="title">Dashboard</h1>
	</div>

	<div class="columns is-centered">
		<div class="column is-two-thirds-desktop">
			<DomainRecordIpChanges />
		</div>
	</div>
</div>

<style lang="scss">
	.title {
		margin: 0;
		font-size: 1.5rem;
		font-weight: 400;
		line-height: 2.5rem;
	}
</style>
