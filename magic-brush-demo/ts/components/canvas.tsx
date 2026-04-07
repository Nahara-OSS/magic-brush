import { ComponentChildren } from "preact";
import "./canvas.css";

export function CanvasFrame({
    width,
    height,
    scaleFactor,
    children,
}: {
    width: number;
    height: number;
    scaleFactor: number;
    children: ComponentChildren;
}) {
    return (
        <div
            className="canvas-frame"
            style={{
                "--canvas-width": width,
                "--canvas-height": height,
                "--scale-factor": scaleFactor,
            }}
        >
            <div className="horizontal ruler"></div>
            <div className="horizontal resizer"></div>
            <div className="vertical ruler"></div>
            <div className="vertical resizer"></div>
            <div className="corner resizer"></div>
            <div className="inner">
                {children}
            </div>
        </div>
    );
}
