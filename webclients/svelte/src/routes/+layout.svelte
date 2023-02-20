<script>
	import '../app.scss';
	import { QueryClient, QueryClientProvider } from '@tanstack/svelte-query';
	import PageHeader from '../components/PageHeader.svelte';

	const queryClient = new QueryClient();
</script>

<QueryClientProvider client={queryClient}>
	<PageHeader />
	<main class="main-body">
		<slot />
	</main>
</QueryClientProvider>
