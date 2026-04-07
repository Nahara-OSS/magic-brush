import { render } from "preact";
import { useEffect, useRef, useState } from "preact/hooks";
import { Graph } from "./ts/types/graph.ts";
import { GraphEditor } from "./ts/components/graph.tsx";
import { BitmapEditor } from "./ts/components/bitmap.tsx";
import { CanvasFrame } from "./ts/components/canvas.tsx";
import { Runtime } from "./loader.ts";
import "./index.css";

const runtime = await Runtime.create();
const doc = runtime.createDocument("Hello world!", 500, 500);
doc.insertLayer(0, "Layer 1").free();
console.log(runtime, doc);

function App() {
    const [width, setWidth] = useState(500);
    const [height, setHeight] = useState(500);
    const canvasRef = useRef<HTMLCanvasElement>(null);

    useEffect(() => {
        if (!canvasRef.current) return;

        const surface = runtime.createHtmlSurface(canvasRef.current);
        surface.configure();

        return () => {
            surface.free();
        };
    }, []);

    return (
        <>
            <div className="document">
                <div className="menu-bar">
                    <button type="button">
                        <span className="label">File</span>
                    </button>
                    <button type="button">
                        <span className="label">Edit</span>
                    </button>
                    <button type="button">
                        <span className="label">Help</span>
                    </button>
                </div>
                <div className="section">
                    <span>
                        <b>Nahara's Magic Brush</b>
                    </span>
                </div>
                <div className="section">
                    <span>
                        <b>Document info</b>
                    </span>
                </div>
                <div className="section">
                    <span>
                        <b>Layers</b>
                    </span>
                </div>
            </div>
            <div className="canvas-area">
                <div className="inner">
                    <CanvasFrame width={width} height={height} scaleFactor={1}>
                        <canvas
                            width={width}
                            height={height}
                            ref={canvasRef}
                        />
                    </CanvasFrame>
                </div>
            </div>
            <div className="brush-editor">
                <div className="section">
                    <span>
                        <b>Brush Editor</b>
                    </span>
                    <div className="horizontal-button-group">
                        <button type="button">
                            <span className="label">Save</span>
                        </button>
                        <button type="button">
                            <span className="label">Load</span>
                        </button>
                        <button type="button">
                            <span className="label">Reset</span>
                        </button>
                    </div>
                    <select>
                        <option value="stamp">Stamp-based</option>
                        <option value="strip">Strip-based</option>
                    </select>
                </div>
                <div className="section">
                    <span>
                        <b>Stamp-based brush</b>
                    </span>
                </div>
            </div>
        </>
    );
}

render(<App />, document.body.querySelector("div#inner")!);
