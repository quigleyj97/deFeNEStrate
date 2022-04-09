import * as React from 'react';
import * as ReactDOM from "react-dom";

import "./views/nes";
import App from "./views/app";

import "./index.scss";

var mountNode = document.getElementById("app");
ReactDOM.render(<App />, mountNode);
