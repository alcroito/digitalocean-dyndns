<script>
	import '../app.scss';
	import { QueryClient, QueryClientProvider } from '@tanstack/svelte-query';
	import { SvelteQueryDevtools } from '@tanstack/svelte-query-devtools';
	import PageHeader from '../components/PageHeader.svelte';

	const queryClient = new QueryClient();
</script>

<QueryClientProvider client={queryClient}>
	<PageHeader />
	<main class="main-body">
		<slot />
	</main>
	<SvelteQueryDevtools initialIsOpen={false} />
</QueryClientProvider>
