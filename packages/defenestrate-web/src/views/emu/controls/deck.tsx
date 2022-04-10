import React from "react";

import "./deck.scss";

interface IControlDeckProps {
    // state params
    isEmulating: boolean,

    // callbacks
    onToggleEmulation: () => void;
    onLoad: () => void;
}

const ControlDeck: React.FC<IControlDeckProps> = ({
    isEmulating,
    onToggleEmulation,
    onLoad
}) => {
    return (<div id="control-deck">
        <button className="ui-btn"
            onClick={() => onToggleEmulation()}>
            {isEmulating ? "Stop" : "Play"}
        </button>
        <button className="ui-btn"
            onClick={() => onLoad()}>
            Load NESTEST
        </button>
    </div>)
}

export default ControlDeck;
