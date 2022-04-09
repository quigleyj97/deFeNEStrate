import React from "react";
import { useEffect, useRef, useState } from "react";
import { HTMLNesEmulatorElement } from "../nes";
import ControlDeck from "./controls/deck";

const Emulator: React.FC = () => {
    const [isEmulating, setIsEmulating] = useState(false);
    const ref = useRef<HTMLNesEmulatorElement>(null);
    useEffect(() => {
        if (!ref.current || !isEmulating) return;
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
            alert("100 frames completed in " + (end - start) + "ms");
            setIsEmulating(false);
        });
    }, [isEmulating, ref]);

    return (<div id="Emulator">
        <nes-emulator ref={ref}></nes-emulator>
        <ControlDeck isEmulating={isEmulating}
            onToggleEmulation={() => setIsEmulating(!isEmulating)} />
    </div>
    )
}

export default Emulator;
