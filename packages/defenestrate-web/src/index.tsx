import * as React from 'react';
import * as ReactDOM from "react-dom";

var mountNode = document.getElementById("app");
ReactDOM.render(<div>'ello, world!</div>, mountNode);

import("../../defenestrate-core/pkg/index.js").then(async (module) => {
    const nestest = await (await fetch("./nestest.nes"))
        .arrayBuffer();
    const buf = new Uint8Array(nestest);
    module.hello(buf);
});
