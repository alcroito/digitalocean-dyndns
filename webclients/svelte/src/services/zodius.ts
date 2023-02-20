import { api as zodiusClientDefault, createApiClient } from '../generated/zodiusClient';

const createClient = () => {
	let zodiusClient = zodiusClientDefault;
	// Set in non-git-controlled .env.development file in the base dir of the web app
	// pointing to http://localhost:8095/. Use for API requests when running
	// vite dev.
	if (import.meta.env.VITE_SERVER_API_BASE_URL) {
		zodiusClient = createApiClient(import.meta.env.VITE_SERVER_API_BASE_URL);
	}
	return zodiusClient;
};

export const zodiusClient = createClient();
