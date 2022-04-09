import React from "react";

import "./deck.scss";

interface IControlDeckProps {
    // state params
    isEmulating: boolean,

    onToggleEmulation: () => void;
}

const ControlDeck: React.FC<IControlDeckProps> = ({
    isEmulating,
    onToggleEmulation
}) => {
    return (<div id="control-deck">
        <button className="ui-btn"
            onClick={() => onToggleEmulation()}>
            {isEmulating ? "Stop" : "Play"}
        </button>
    </div>)
}

export default ControlDeck;
