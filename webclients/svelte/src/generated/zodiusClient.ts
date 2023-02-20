import { makeApi, Zodios, type ZodiosOptions } from '@zodios/core';
import { z } from 'zod';

const DomainRecordIpChange = z.object({
	attempt_date: z.string(),
	domain_record_id: z.number().int(),
	id: z.number().int(),
	name: z.string(),
	set_ip: z.string(),
	success: z.boolean()
});
const DomainRecordIpChanges = z.object({ changes: z.array(DomainRecordIpChange) });

export const schemas = {
	DomainRecordIpChange,
	DomainRecordIpChanges
};

const endpoints = makeApi([
	{
		method: 'get',
		path: '/api/v1/domain_record_ip_changes',
		description: `List all recent domain record ip changes`,
		requestFormat: 'json',
		response: DomainRecordIpChanges,
		errors: [
			{
				status: 'default',
				schema: z.object({ GenericError: z.string() })
			}
		]
	},
	{
		method: 'get',
		path: '/docs/',
		description: `This documentation page.`,
		requestFormat: 'json',
		response: z.void(),
		errors: [
			{
				status: 'default',
				schema: z.object({ GenericError: z.string() })
			}
		]
	}
]);

export const api = new Zodios(endpoints);

export function createApiClient(baseUrl: string, options?: ZodiosOptions) {
	return new Zodios(baseUrl, endpoints, options);
}
