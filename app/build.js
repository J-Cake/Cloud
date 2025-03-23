import * as fs from 'node:fs/promises';
import esbuild from 'esbuild';

const result = await esbuild.build({
	entryPoints: ["src/main.tsx"],
	bundle: true,
	sourcemap: true,
	platform: "browser",
	format: "esm",
	outdir: "build",
	splitting: true,
	treeShaking: true,
	minify: true,

	plugins: [{
		name: "sql",
		setup(build)  {
			build.onResolve({ filter: /^.*\.sql$/ }, args => ({
				path: args.path,
				namespace: "sql-ns"
			}));

			build.onLoad({ filter: /^.*\.sql$/, namespace: "sql-ns" }, args => ({
				contents: `
					const join = (...paths) => paths
						.map(i => i.replaceAll(/^\\.\\//g, ''))
						.join("/");
					export default config => async function(params) {
						return fetch(join(config.baseUrl, decodeURIComponent("${encodeURIComponent(args.path)}")))
							.then(res => res.json())
					}`,
				loader: "js"
			}))
		}
	}]
});

console.log(result);

for await (const file of await fs.readdir("public/translations"))
	await fs.copyFile(`public/translations/${file}`, `build/${file}`);