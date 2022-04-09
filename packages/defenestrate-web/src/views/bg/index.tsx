import React from "react";
import "./bg.scss";

// A helper class to render the background and tie all BG-related rules and assets together
const AppBackground: React.FC = () => {
    return (<div id="app-background">
        <div id="grid"></div>
    </div>)
};

export default AppBackground;

export const TitleCard: React.FC = () => {
    return <div className="ui-display-txt">deFeNEStrate</div>
}

export const Gutter: React.FC = ({ children }) => {
    return (<div className="gutter">
        <span className="gutter-spacer"></span>
        <div className="gutter-content">
            {children}
        </div>
        <span className="gutter-spacer"></span>
    </div>);
}