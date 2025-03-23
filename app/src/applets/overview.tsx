import React from 'react';
import * as router from 'react-router';

import { user } from '../main.js';
import $ from "../localisation.js";

export default function Overview() {
	const login = React.useContext(user)!;

	return <>
		{$`Wilkommen ${login.user.displayName}!`}
	</>;
}