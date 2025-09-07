<script lang="ts">
	import { VersionStore } from '../stores/VersionStore';
	import { getContext } from 'svelte';
	import type { QueryClient } from '@tanstack/svelte-query';
	import type { z } from 'zod';
	import type { schemas } from '../generated/zodiosClient';

	type VersionResponse = z.infer<typeof schemas.VersionResponse>;

	// GitHub repository base URL
	const GITHUB_REPO = 'https://github.com/alcroito/digitalocean-dyndns';

	const queryClient: QueryClient = getContext('queryClient');
	const versionStore = new VersionStore(queryClient);
	const versionQuery = versionStore.query;

	$: version = $versionQuery.data as VersionResponse;
</script>

<div class="version-display">
	{#if $versionQuery.isLoading}
		<div class="is-loading">
			<span class="loader"></span>
		</div>
	{:else if $versionQuery.error}
		<div class="notification is-danger is-light">
			<p class="has-text-danger">
				Failed to load version: {versionStore.handleError($versionQuery.error)}
			</p>
		</div>
	{:else if version}
		<div class="version-info">
			<div class="version-main">
				<span class="version-label">Version:</span>
				<!-- eslint-disable svelte/no-navigation-without-resolve -->
				<a
					class="version-value version-link"
					href="{GITHUB_REPO}/releases/tag/v{version.version}"
					target="_blank"
					rel="external noopener noreferrer"
					data-sveltekit-reload
				>
					{version.version}
				</a>
				<!-- eslint-enable svelte/no-navigation-without-resolve -->
			</div>
			{#if version.git_sha}
				<div class="version-detail">
					<span class="version-label">Commit:</span>
					<!-- eslint-disable svelte/no-navigation-without-resolve -->
					<a
						class="version-value git-sha commit-link"
						href="{GITHUB_REPO}/commit/{version.git_sha}"
						target="_blank"
						rel="external noopener noreferrer"
						data-sveltekit-reload
					>
						{version.git_sha.slice(0, 8)}
					</a>
					<!-- eslint-enable svelte/no-navigation-without-resolve -->
				</div>
			{/if}
			{#if version.git_branch}
				<div class="version-detail">
					<span class="version-label">Branch:</span>
					<span class="version-value">{version.git_branch}</span>
				</div>
			{/if}
			<div class="version-detail">
				<span class="version-label">Built:</span>
				<span class="version-value build-date" title={version.build_timestamp}>
					{new Date(version.build_timestamp).toLocaleDateString()}
				</span>
			</div>
		</div>
	{/if}
</div>

<style lang="scss">
	.version-display {
		padding: 0.5rem;
	}

	.version-info {
		display: flex;
		flex-wrap: wrap;
		gap: 1rem;
		align-items: center;
		font-size: 0.875rem;
		color: #6e7687;
	}

	.version-main {
		font-weight: 500;
	}

	.version-detail {
		display: flex;
		align-items: center;
		gap: 0.25rem;
	}

	.version-label {
		color: #9aa0ac;
		font-weight: 400;
	}

	.version-value {
		color: #5eba00;
		font-weight: 500;
	}

	.git-sha {
		font-family: monospace;
		background: rgba(94, 186, 0, 0.1);
		padding: 0.125rem 0.25rem;
		border-radius: 3px;
		font-size: 0.8125rem;
	}

	.build-date {
		cursor: help;
		text-decoration: underline;
		text-decoration-style: dotted;
		text-underline-offset: 2px;
	}

	.version-link,
	.commit-link {
		color: #5eba00;
		text-decoration: none;
		transition: color 0.2s ease;
	}

	.version-link:hover,
	.commit-link:hover {
		color: #4b9400;
		text-decoration: underline;
	}

	.version-link:visited,
	.commit-link:visited {
		color: #5eba00;
	}

	.is-loading {
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 1rem;
	}

	.loader {
		width: 16px;
		height: 16px;
		border: 2px solid #f3f3f3;
		border-top: 2px solid #5eba00;
		border-radius: 50%;
		animation: spin 1s linear infinite;
	}

	@keyframes spin {
		0% {
			transform: rotate(0deg);
		}
		100% {
			transform: rotate(360deg);
		}
	}
</style>
