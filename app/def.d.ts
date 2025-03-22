declare module "*.sql" {
	type Parameters = Record<string, string>;
	type Config = typeof import("./src/api.ts").config;

	interface ApiResponse<T> {
		success: boolean;
		data: T
	}

	export default function<T>(config: Config): (params: Parameters) => Promise<ApiResponse<T>>;
}