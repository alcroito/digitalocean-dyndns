import type { AxiosError } from 'axios';
import axios from 'axios';
import { createQuery, QueryClient } from '@tanstack/svelte-query';
import { defer, delay, firstValueFrom, TimeoutError } from 'rxjs';
import type { z } from 'zod';
import type { schemas } from '../generated/zodiosClient';
import { zodiosClient } from '../services/zodios';

type VersionResponse = z.infer<typeof schemas.VersionResponse>;
type VersionError = Error | AxiosError | TimeoutError;

export class VersionStore {
	public query;

	constructor(public queryClient: QueryClient) {
		this.query = createQuery<VersionResponse, VersionError>({
			queryKey: ['version'],
			queryFn: this.queryFn,
			retry: 1,
			// Avoid macOS + Chrome offline bug https://github.com/TanStack/query/issues/5679
			networkMode: 'offlineFirst',
			// Cache for a longer time since version rarely changes
			staleTime: 5 * 60 * 1000, // 5 minutes
			gcTime: 30 * 60 * 1000 // 30 minutes
		});
	}

	private queryFn() {
		const route = '/api/v1/version';
		const obs = defer(() => zodiosClient.get(route)).pipe(delay(100));
		const promise = firstValueFrom(obs);
		return promise;
	}

	public reset() {
		this.queryClient.resetQueries({ queryKey: ['version'], exact: true });
	}

	public handleError(error: VersionError): string {
		// Allows doing something special depending on the error type.
		if (axios.isAxiosError(error)) {
			return error.message;
		} else if (error instanceof TimeoutError) {
			return 'Failed to fetch version due to timeout';
		}
		return error.message;
	}
}
