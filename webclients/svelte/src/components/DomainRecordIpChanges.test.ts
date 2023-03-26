import { render, fireEvent, screen, waitFor } from '@testing-library/svelte';
import { QueryClient } from '@tanstack/query-core';
import { QueryClientProvider } from '@tanstack/svelte-query';
// import * as foo from '@tanstack/svelte-query';

import DomainRecordIpChanges from './DomainRecordIpChanges.svelte';
import TestComponentParentWrapper from './TestComponentParentWrapper.svelte';

describe('Test DomainRecordIpChanges.svelte', async () => {
	it('Initial load should be loading', async () => {
		// let client = new QueryClient();
		// client.mount()
		// foo.set
		// setQueryClientContext(client);
		// render(QueryClientProvider);
		// render(DomainRecordIpChanges);
		render(TestComponentParentWrapper, {
			props: {
				parentComponent: QueryClientProvider,
				childComponent: DomainRecordIpChanges
			}
		});
		// expect(screen.getByText('hello')).toBeInTheDocument();
		await waitFor(() => {
			expect(screen.getByText('GreatSuccess', {exact: false})).toBeInTheDocument();
		}, {timeout:3000});
		// expect(screen.getByText('0')).toBeInTheDocument();
		// client.unmount()
	});
	//   it('Test decrease', async () => {
	//     render(Counter);
	//     const decreaseButton = screen.getByLabelText('Decrease the counter by one');
	//     // Decrease by two
	//     await fireEvent.click(decreaseButton);
	//     await fireEvent.click(decreaseButton);
	//     // Wait for animation
	//     const counter = await screen.findByText('-2');
	//     expect(counter).toBeInTheDocument();
	//   });
	//   it('Test increase', async () => {
	//     render(Counter);
	//     const increaseButton = screen.getByLabelText('Increase the counter by one');
	//     // Increase by one
	//     await fireEvent.click(increaseButton);
	//     // Wait for animation
	//     const counter = await screen.findByText('1');
	//     expect(counter).toBeInTheDocument();
	//   });
});
