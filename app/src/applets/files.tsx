import * as React from 'react';
import * as router from 'react-router';

import Loading from "../components/loading.js";
import $ from "../localisation.js";
import * as api from "../api.js";

interface Props {
	path?: string
}

export const fileMap: Map<string, api.FileEntry | (api.DirEntry & { contents?: api.DirContents[] })> = new Map();

export default function Files(props: Props) {
	const [query, setQuery] = router.useSearchParams();
	const path = React.useMemo(() => query.get("path") ?? '/', [query]);

	const [index, setIndex] = React.useState<api.DirContents[] | null>(null);

	React.useEffect(() => void Array.fromAsync(api.readDir(path, 1))
		.then(index => setIndex(index)), [path]);

	if (index)
		return <>
			{path}
			<ul>
				{index.map(i => 'file' in i ? <li key={i.file}>{i.file}</li> : <li key={i.dir}>{i.dir}</li>)}
			</ul>
		</>;
	else return <Loading />;
}