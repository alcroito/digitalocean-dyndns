import { api as zodiosClientDefault, createApiClient } from '../generated/zodiosClient';

const createClient = () => {
	let zodiosClient = zodiosClientDefault;
	// Set in non-git-controlled .env.development file in the base dir of the web app
	// pointing to http://localhost:8095/. Use for API requests when running
	// vite dev.
	if (import.meta.env.VITE_SERVER_API_BASE_URL) {
		zodiosClient = createApiClient(import.meta.env.VITE_SERVER_API_BASE_URL);
	}
	return zodiosClient;
};

export const zodiosClient = createClient();
