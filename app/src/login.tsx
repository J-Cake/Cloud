import Button from "./components/button.js";
import * as api from './api.js';

export default function Login(props: {}) {
	return <>
		<h1>{"Login!"}</h1>
		<p>{"Log in to proceed"}</p>
		<a href={api.getGithubLoginUrl()}>
			<Button label={"Log In"} icon={'\uEDCA'} onClick={e => {}}/>
		</a>
	</>;
}