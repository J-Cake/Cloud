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

export type FileEntry = {
	file: string,
	size: number,
	modified: Date,
	created: Date
};
export type DirEntry = {
	dir: string
};

export type DirContents = FileEntry | DirEntry;

export async function* readDir(dir: string, depth: number = 100): AsyncGenerator<DirContents> {
	const url = new URL(config.apiLocation + "/system");
	url.searchParams.set("command", "file::lsdir");
	url.searchParams.set("args", [dir, "--depth", depth].join(';'));

	const token = await Promise.resolve(window.localStorage.getItem("token"))
		.then(res => !res ? Promise.reject("No token") : Promise.resolve(res))
		.then(token => JSON.parse(token) as string);

	const reader = await fetch(url, {
		method: "POST",
		headers: {
			Authorization: `Bearer ${token}`
		}
	}).then(res => res.body?.pipeThrough(new TextDecoderStream()));

	if (!reader)
		return Promise.reject("Failed to read directory");

	async function* read(reader: ReadableStream) {
		const buffer: string[] = [];

		for await (const chunk of reader) {
			const chunks = buffer.splice(0, buffer.length)
				.concat(chunk)
				.join("")
				.split("\n");

			buffer.push(chunks.pop()!);

			for (const chunk of chunks)
				if (chunk.trim().length > 0)
					yield JSON.parse(chunk);
		}

		for (const chunk of buffer.splice(0, buffer.length))
			if (chunk.trim().length > 0)
				yield JSON.parse(chunk);
	}

	for await (const dirent of read(reader))
		if ("File" in dirent)
			yield {
				file: dirent.File.path,
				created: new Date(dirent.File.created.secs_since_epoch * 1000),
				modified: new Date(dirent.File.modified.secs_since_epoch * 1000),
				size: dirent.File.size
			};
		else if ("Dir" in dirent)
			yield { dir: dirent.Dir };
}

export async function loadUser(): Promise<LoginResult | null> {
	const token: string = await Promise.resolve(window.localStorage.getItem("token"))
		.then(token => !token ? Promise.reject() : token)
		.then(str => JSON.parse(str))
		.catch(_ => null);

	if (token)
		return await fetch(config.userUrl, { headers: { Authorization: `Bearer ${token}` } })
			.then(res => {
				if (res.ok)
					return res.json();

				if (res.status == 401) // Token is invalid
					window.localStorage.removeItem("token")

				return null;
			});
	else
		return null;
}

export async function collect<T>(iter: AsyncIterable<T>): Promise<T[]> {
	const res: T[] = [];

	for await (const i of iter)
		console.log(i);
		// res.push(i);

	return res;
}