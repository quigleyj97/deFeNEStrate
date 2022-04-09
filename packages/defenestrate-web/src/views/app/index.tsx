import React, { useEffect, useRef } from "react";
import { HTMLNesEmulatorElement } from "../nes";
import AppBackground from "../bg";

import "./app.scss";

const App: React.FC<{}> = ({ }) => {
    const ref = useRef<HTMLNesEmulatorElement>(null);
    useEffect(() => {
        if (!ref.current) return;
        const el = ref.current;
        const nestest = fetch("./nestest.nes").then(res => res.arrayBuffer());
        const init = el.init();
        Promise.all([nestest, init]).then(([rom, _]) => {
            el.loadRom(rom);
            alert("Dismiss to run 100 frames");
            const start = Date.now();
            for (let i = 0; i < 100; i++) {
                el.run_frame();
            }
            const end = Date.now();
            alert("100 frames completed in " + (end - start) + "ms")
        });
    }, [ref]);
    return (
        <>
            <AppBackground />
            <nes-emulator ref={ref}></nes-emulator>
        </>
    );
};

export default App;
