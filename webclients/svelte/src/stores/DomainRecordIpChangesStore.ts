import type { AxiosError } from 'axios';
import axios from 'axios';
import { createQuery, QueryClient } from '@tanstack/svelte-query';
import { defer, delay, firstValueFrom, TimeoutError } from 'rxjs';
import type { z } from 'zod';
import type { schemas } from '../generated/zodiosClient';
import { zodiosClient } from '../services/zodios';

type DomainRecordIpChange = z.infer<typeof schemas.DomainRecordIpChange>;
type DomainRecordIpChanges = {
	changes: DomainRecordIpChange[];
};
type DomainRecordIpChangesError = Error | AxiosError | TimeoutError;

export class DomainRecordIpChangesStore {
	public query;

	constructor(public queryClient: QueryClient) {
		this.query = createQuery<DomainRecordIpChanges, DomainRecordIpChangesError>({
			queryKey: ['domain_record_ip_changes'],
			queryFn: this.queryFn,
			retry: 1,
			// Avoid macOS + Chrome offline bug https://github.com/TanStack/query/issues/5679
			networkMode: 'offlineFirst'
		});
	}

	private queryFn() {
		// TODO: Use msw to mock requests. https://mswjs.io/
		const route = '/api/v1/domain_record_ip_changes';
		const obs = defer(() => zodiosClient.get(route)).pipe(delay(500));
		const promise = firstValueFrom(obs);
		return promise;
	}

	public reset() {
		this.queryClient.resetQueries({ queryKey: ['domain_record_ip_changes'], exact: true });
	}

	public handleError(error: DomainRecordIpChangesError): string {
		// Allows doing something special depending on the error type.
		if (axios.isAxiosError(error)) {
			if (error.message) {
				return error.message;
			} else if (error.code) {
				return "Axios error: " + error.code;
			} else {
				console.error("Unknown axios error", error);
				return "Unknown axios error";
			}
		} else if (error instanceof TimeoutError) {
			return 'Failed to fetch data due to timeout';
		}
		return error.message;
	}
}
