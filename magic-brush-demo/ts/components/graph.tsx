import { Graph, GraphPoint } from "../types/graph.ts";
import { useState } from "preact/hooks";
import "./graph.css";

export function GraphEditor({ graph, setGraph }: {
    graph: Graph;
    setGraph: (updater: (graph: Graph) => Graph) => unknown;
}) {
    return (
        <GraphEditorInner
            key={graph}
            graph={graph}
            setGraph={setGraph}
        />
    );
}

function GraphEditorInner({ graph, setGraph }: {
    graph: Graph;
    setGraph: (updater: (graph: Graph) => Graph) => unknown;
}) {
    const [points, setPoints] = useState(() => graph.map((point, index) => ({ index, point })));
    const [lastClick, setLastClick] = useState<number | null>(null);
    const sorted = points.sort((a, b) => a.point[0] - b.point[0]);

    const onPointerDownInner = (e: PointerEvent) => {
        if (e.target != e.currentTarget) return;

        if (lastClick != null && e.timeStamp - lastClick < 500) {
            const bounds = (e.currentTarget as HTMLElement).getBoundingClientRect();
            const x = (e.clientX - bounds.x) / bounds.width;
            const y = 1 - (e.clientY - bounds.y) / bounds.height;

            setLastClick(null);
            setGraph((graph) => {
                const array = [...graph, [x, y] as GraphPoint];
                array.sort((a, b) => a[0] - b[0]);
                return array;
            });
        } else {
            setLastClick(e.timeStamp);
        }
    };

    return (
        <div className="graph-editor">
            <div className="inner" onPointerDown={onPointerDownInner}>
                <svg className="lines">
                    <line
                        x1="0%"
                        y1="100%"
                        x2={`${(sorted[0]?.point[0] ?? 100) * 100}%`}
                        y2={`${100 - (sorted[0]?.point[1] ?? 100) * 100}%`}
                    />
                    {sorted.map(({ index, point }, i) =>
                        i >= sorted.length - 1 ? null : (
                            <line
                                key={index}
                                x1={`${point[0] * 100}%`}
                                y1={`${100 - point[1] * 100}%`}
                                x2={`${sorted[i + 1].point[0] * 100}%`}
                                y2={`${100 - sorted[i + 1].point[1] * 100}%`}
                            />
                        )
                    )}
                    <line
                        x1="100%"
                        y1="0%"
                        x2={`${(sorted.at(-1)?.point[0] ?? 0) * 100}%`}
                        y2={`${100 - (sorted.at(-1)?.point[1] ?? 0) * 100}%`}
                    />
                </svg>
                {sorted.map(({ index, point }) => (
                    <GraphPointHandle
                        key={index}
                        point={point}
                        setPoint={(updater) => {
                            setPoints((points) => (points.map(({ index: i, point }) => {
                                if (index == i) return { index: i, point: updater(point) };
                                return { index: i, point };
                            })));
                        }}
                        onPointerUp={() => {
                            setPoints((points) => {
                                const newGraph = points
                                    .sort((a, b) => a.point[0] - b.point[0])
                                    .map(({ point }) => point);

                                setGraph(() => newGraph);
                                return points;
                            });
                        }}
                    />
                ))}
            </div>
        </div>
    );
}

function GraphPointHandle({ point, setPoint, onPointerUp }: {
    point: GraphPoint;
    setPoint: (updater: (point: GraphPoint) => GraphPoint) => unknown;
    onPointerUp: (event: PointerEvent) => unknown;
}) {
    const [dragging, setDragging] = useState(false);

    return (
        <div
            className="graph-point"
            style={{
                "--point-input": point[0],
                "--point-output": point[1],
            }}
            onPointerDown={(e) => {
                const pointerId = e.pointerId;
                const parent = e.currentTarget.parentElement;
                const bounds = parent?.getBoundingClientRect();
                const [ix, iy] = point;
                const cx = e.clientX;
                const cy = e.clientY;

                if (dragging || bounds == null) return;
                setDragging(true);

                const onPointerMoveInner = (e: PointerEvent) => {
                    if (e.pointerId != pointerId) return;
                    const dcx = e.clientX - cx;
                    const dcy = e.clientY - cy;
                    const x = Math.max(Math.min(ix + dcx / bounds.width, 1.0), 0.0);
                    const y = Math.max(Math.min(iy - dcy / bounds.height, 1.0), 0.0);
                    setPoint(() => [x, y]);
                };

                const onPointerUpInner = (e: PointerEvent) => {
                    if (e.pointerId != pointerId) return;
                    setDragging(false);
                    onPointerUp(e);
                    document.removeEventListener("pointermove", onPointerMoveInner);
                    document.removeEventListener("pointerup", onPointerUpInner);
                };

                document.addEventListener("pointermove", onPointerMoveInner);
                document.addEventListener("pointerup", onPointerUpInner);
            }}
        >
        </div>
    );
}
