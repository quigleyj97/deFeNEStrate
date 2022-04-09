import React from "react";
import AppBackground, { Gutter, TitleCard } from "../bg";
import Emulator from "../emu";

import "./app.scss";

const App: React.FC<{}> = ({ }) => {
    return (
        <>
            <AppBackground />
            <Gutter>
                <TitleCard />
                <Emulator />
            </Gutter>
        </>
    );
};

export default App;
