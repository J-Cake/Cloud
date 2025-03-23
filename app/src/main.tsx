import React from 'react';
import * as dom from 'react-dom/client';
import * as reactRouter from 'react-router';

import Login from "./login.js";
import Overview from "./applets/overview.js";
import Files from "./applets/files.js";
import Loading from "./components/loading.js";

import * as api from "./api.js";

import $, {Language, loadLanguage} from "./localisation.js";

import '@/main.css';

export const selectedLanguage = React.createContext<Language>('de_DE');
export const user = React.createContext<api.LoginResult | null>(null);

export function applet(path: string, child: React.ReactNode, loader?: reactRouter.RouteObject['loader']): reactRouter.RouteObject {
	return {
		path,
		element: <App>{child}</App>,
		loader
	}
}

export const router = reactRouter.createBrowserRouter([
	applet("/", <Overview />),
	applet("/files", <Files />),
	{
		path: "/oauth-callback",
		element: <App />,
		async loader(route) {
			const code = new URL(route.request.url).searchParams.get("code");

			if (!code) return;

			const url = new URL(api.config.loginUrl);
			url.searchParams.set("code", code);

			const user = await fetch(url)
				.then(res => res.json() as Promise<api.LoginResult>);

			if (!user.success)
				return null;

			window.localStorage.setItem("token", JSON.stringify(user.token));
			window.history.replaceState(null, "", "/app");

			return user;
		}
	}
], {
	basename: "/app"
});

export default async function main(root: HTMLElement) {
	await loadLanguage("en_AU", "de_DE");

	dom.createRoot(root)
		.render(<reactRouter.RouterProvider router={router} />);
}

export function App(props: React.PropsWithChildren<{}>) {
	const [state, setState] = React.useState<api.LoginResult | null | { success: false }>(null)

	if (!state)
		api.loadUser().then(setState);

	if (state && state.success)
		return <user.Provider value={state}>
			{props.children}
		</user.Provider>;
	else if (!state)
		return <Loading />;
	else
		return <Login/>;
}