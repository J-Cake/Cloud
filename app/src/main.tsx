import React from 'react';
import * as dom from 'react-dom/client';
import * as reactRouter from 'react-router';

import Welcome from "./welcome.js";
import * as api from "./api.js";

import '@/main.css';

export const router = reactRouter.createBrowserRouter([
	{
		path: "/",
		element: <App />,
		async loader(router) {
			const token: string = await Promise.resolve(window.localStorage.getItem("token"))
				.then(token => !token ? Promise.reject() : token)
				.then(str => JSON.parse(str))
				.catch(_ => null);

			if (token)
				return await fetch(api.config.userUrl, { headers: { Authorization: `Bearer ${token}` } });
		}
	},
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

			return user;
		}
	}

], {
	basename: "/app"
});

export default async function main(root: HTMLElement) {
	dom.createRoot(root)
		.render(<reactRouter.RouterProvider router={router} />);
}

export function App() {
	const data = reactRouter.useLoaderData<api.LoginResult>();

	if (data)
		return <h1>{`You are logged in as ${data.user.displayName}`}</h1>;
	else
		return <Welcome/>;
}