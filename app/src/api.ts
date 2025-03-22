import config from '../config.json';

export interface Config {
	baseUrl: string;
}

export interface LoginResult {
	success: true,
	token: string,
	expiry: Date,
	user: {
		email: string,
		displayName: string
	}
}

export function getGithubLoginUrl(): string {
	return Object.entries(config)
		.reduce((acc, [a, i]) => acc.replaceAll(`$${a}`, i), config.oauthService);
}

export { default as config } from '../config.json';