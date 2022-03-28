import * as React from 'react';
import * as ReactDOM from "react-dom";

var mountNode = document.getElementById("app");
ReactDOM.render(<div>'ello, world!</div>, mountNode);

import("../../defenestrate-core/pkg/index.js").then(async (module) => {
    const nestest = await (await fetch("./nestest.nes"))
        .arrayBuffer();
    const buf = new Uint8Array(nestest);
    module.init_debug_hooks();
    // module.hello(buf);
    const emulator = new module.NesEmulator(buf);
    console.log("Emulator: ", emulator);

    alert("Dismiss");

    console.log("Frame data: ", emulator.step_frame());
    emulator.free();
});
