import * as React from 'react';

import {selectedLanguage} from "./main.js";

export interface Translation {
	[Key: string]: string[]
}

export const loadedLanguages: Record<string, Translation> = {
	en_AU: await fetch("/static/en_AU.json")
		.then(res => res.json()),
	de_DE: await fetch("/static/de_DE.json")
		.then(res => res.json()),
};

export type Language = keyof typeof loadedLanguages;
export const referenceLanguage: Language = 'en_AU';

const translated: Map<string, {
	[Key in Language]: (...values: string[]) => string
}> = new Map();

export default function $(segments: TemplateStringsArray, ...interpolated: any[]) {
	const language = React.useContext(selectedLanguage);

	const joined = segments
		.reduce((acc, i, a) => acc.concat(i, String(interpolated[a])), [] as string[])
		.slice(0, -1)
		.join("");

	if (!translated.has(joined)) {
		const lowerJoined = joined.toLowerCase();

		for (const [key, template] of Object.entries(loadedLanguages[language])
			.filter(([_, value]) => value.length - 1 == interpolated.length)) {

			const reference = loadedLanguages[referenceLanguage][key];

			if (reference.slice(1)
				.reduce((acc, slot, index) => acc.replaceAll(`$${slot}`, String(interpolated[index])), reference[0])
				.toLowerCase() == lowerJoined) {

				translated.set(joined, Object.assign(translated.get(joined) ?? {
					[referenceLanguage]: (...values: string[]) => reference
						.slice(1)
						.reduce((acc, slot, index) => acc.replaceAll(`$${template[1 + index]}`, slot), reference[0])
				}, {
					[language]: (...values: string[]) => {
						const template = loadedLanguages[language][key];
						return values.reduce((acc, slot, index) => acc.replaceAll(`$${template[1 + index]}`, slot), template[0]);
					}
				}));

				break;
			}
		}
	}

	return <span>{(translated.get(joined)?.[language])?.(...interpolated) ?? joined}</span>;
}

export async function loadLanguage(...languages: string[]) {
	await Promise.all(languages.filter(i => !loadedLanguages[i])
		.map(language => fetch(`/static/${language}.json`)
			.then(res => res.json())
			.then(lang => loadedLanguages[language] = lang)))
}