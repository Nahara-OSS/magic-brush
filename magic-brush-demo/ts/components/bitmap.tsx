import { useState } from "preact/hooks";
import "./bitmap.css";

export function BitmapEditor({ width, height, data }: {
    width: number;
    height: number;
    data: Uint8Array;
}) {
    const [color, setColor] = useState(0.5);
    const [drawing, setDrawing] = useState<number | null>(null);

    return (
        <div className="bitmap-editor">
            <table
                className="bitmap-view"
                onPointerDown={(e) => {
                    const pointerId = e.pointerId;
                    setDrawing(pointerId);

                    const onPointerUp = (e: PointerEvent) => {
                        if (e.pointerId != pointerId) return;
                        setDrawing(null);
                        document.removeEventListener("pointerup", onPointerUp);
                    };

                    document.addEventListener("pointerup", onPointerUp);
                }}
            >
                <tbody>
                    {new Array(height).fill(0).map((_, y) => (
                        <tr key={y}>
                            {new Array(width).fill(0).map((_, x) => (
                                <td
                                    key={x}
                                    style={{ "--value": data[x + y * width] / 255 }}
                                    onPointerDown={(e) => {
                                        if (e.button == 2) {
                                            setColor(data[x + y * width] / 255);
                                            e.preventDefault();
                                            e.stopPropagation();
                                            return;
                                        }

                                        data[x + y * width] = Math.floor(color * 255);
                                        e.currentTarget.style.setProperty("--value", `${data[x + y * width] / 255}`);
                                    }}
                                    onPointerMove={(e) => {
                                        if (drawing != e.pointerId) return;
                                        data[x + y * width] = Math.floor(color * 255);
                                        e.currentTarget.style.setProperty("--value", `${data[x + y * width] / 255}`);
                                    }}
                                    onContextMenu={(e) => {
                                        e.preventDefault();
                                    }}
                                />
                            ))}
                        </tr>
                    ))}
                </tbody>
            </table>
            <div
                className="color-picker"
                onPointerDown={(e) => {
                    const pointerId = e.pointerId;
                    const bounds = e.currentTarget.getBoundingClientRect();

                    const onPointerMove = (e: PointerEvent) => {
                        if (e.pointerId != pointerId) return;
                        setColor(Math.max(Math.min((e.clientX - bounds.x) / bounds.width, 1.0), 0.0));
                    };

                    const onPointerUp = (e: PointerEvent) => {
                        if (e.pointerId != pointerId) return;
                        document.removeEventListener("pointermove", onPointerMove);
                        document.removeEventListener("pointerup", onPointerUp);
                    };

                    document.addEventListener("pointermove", onPointerMove);
                    document.addEventListener("pointerup", onPointerUp);
                }}
            >
                <div className="selector" style={{ "--value": color }} />
            </div>
        </div>
    );
}
