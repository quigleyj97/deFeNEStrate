import React from "react";
import { useEffect, useRef, useState } from "react";
import { HTMLNesEmulatorElement } from "../nes";
import { usePrevious } from "../../utils/hooks";
import ControlDeck from "./controls/deck";

const Emulator: React.FC = () => {
    const [isEmulating, setIsEmulating] = useState(false);
    const wasEmulating = usePrevious(isEmulating);
    const ref = useRef<HTMLNesEmulatorElement>(null);
    useEffect(() => {
        if (!ref.current) return;

        if (wasEmulating && !isEmulating) {
            ref.current.haltEmulation();
        } else if (!wasEmulating && isEmulating) {
            ref.current.beginOrResumeEmulation();
        }
    }, [isEmulating, ref]);

    return (<div id="Emulator">
        <nes-emulator ref={ref}></nes-emulator>
        <ControlDeck isEmulating={isEmulating}
            onToggleEmulation={() => setIsEmulating(!isEmulating)}
            onLoad={() => {
                if (!ref.current) return;
                const el = ref.current;
                const nestest = fetch("./nestest.nes").then(res => res.arrayBuffer());
                const init = el.init();
                Promise.all([nestest, init]).then(([rom, _]) => {
                    el.loadRom(rom);
                    alert("Emulator ready");
                });
            }} />
    </div>
    )
}

export default Emulator;
