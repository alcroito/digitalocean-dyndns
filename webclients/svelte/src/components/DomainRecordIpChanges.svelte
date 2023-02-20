<script lang="ts">
	import { DomainRecordIpChangesStore } from '../stores/DomainRecordIpChangesStore';
	import dayjs from 'dayjs';
	import relativeTime from 'dayjs/plugin/relativeTime';
	import { useQueryClient } from '@tanstack/svelte-query';

	dayjs.extend(relativeTime);

	let store = new DomainRecordIpChangesStore(useQueryClient());
	let query = store.query;
</script>

<div class="card">
	<header class="card-header">
		<div class="card-header-title">
			<p class="title has-text-weight-normal is-4">Public IP history</p>
			<p class="subtitle has-text-weight-normal is-6">in descending order</p>
		</div>

		<div class="card-header-icon" class:is-hidden={!$query.isFetching}>
			<div class="loader mr-1" />
		</div>
		<button
			class="refresh button m-3"
			class:is-hidden={$query.isFetching}
			on:click={() => store.reset()}
		>
			<div class="icon">
				<i class="fa-solid fa-arrows-rotate" />
			</div>
		</button>
	</header>
	<div class="card-content px-0 pt-0">
		{#if $query.isLoading}
			<div class="section">
				<div class="has-text-centered">
					<p>fetching data...</p>
				</div>
			</div>
		{:else if $query.isError}
			<div class="section">
				<div class="has-text-centered">
					<p style="color: red">{store.handleError($query.error)}</p>
				</div>
			</div>
		{:else if $query.isSuccess}
			<div class="table-container">
				<table class="table is-fullwidth">
					<thead>
						<tr>
							<th class="has-text-weight-normal has-text-centered">Domain</th>
							<th class="has-text-weight-normal has-text-centered">IP</th>
							<th class="has-text-weight-normal has-text-centered">Modified</th>
						</tr>
					</thead>
					<tbody>
						{#each $query.data.changes as entry}
							<tr>
								<td>{entry.name}</td>
								<td>{entry.set_ip}</td>
								<td>{dayjs(entry.attempt_date).fromNow()}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		{/if}
	</div>
</div>

<style lang="scss">
	.card {
		box-shadow: 0 1px 2px 0 rgb(0 0 0 / 5%);
		border: 1px solid rgba(0, 40, 100, 0.12);

		.loader {
			width: 26px;
			height: 26px;
		}
	}

	.card-header {
		box-shadow: none;
		border-bottom: 1px solid rgba(0, 40, 100, 0.12);
	}

	.card-header-title {
		padding: 0.6rem 1.5rem;
		display: block;

		.title {
			font-size: 1.125rem;
		}

		.subtitle {
			font-size: 0.875rem;
			color: #9aa0ac;
			margin-bottom: 4px;
		}
	}

	.table {
		border: 1px solid rgba(0, 0, 0, 0.1);

		th {
			line-height: normal;
			padding: 5px 5px;
			border-width: 0px 0px 1px 0px;
			border: 0;
			border-right: 1px solid rgba(0, 0, 0, 0.05);
		}

		thead {
			box-shadow: 0 2px 15px 0 rgb(0 0 0 / 15%);
		}

		td {
			border: 0px;
			border-right: 1px solid rgba(0, 0, 0, 0.02);
			padding: 10px 15px;
			line-height: 1.875;

			:first-child {
				width: 100px;
			}
		}

		tbody tr {
			border-bottom: solid 1px rgba(0, 0, 0, 0.05);
		}
	}
</style>
