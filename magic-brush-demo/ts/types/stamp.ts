import { Dynamic } from "./dynamic.ts";
import { Graph } from "./graph.ts";

export type StampBrush = {
    tip: BrushTip;
    spacing: number;
    size: Dynamic;
    flow: Dynamic;
    opacity: Dynamic;
    offset: [Dynamic, Dynamic];
};
export type BrushTip =
    | { circle: BitmapBrushTip }
    | { square: BitmapBrushTip }
    | { bitmap: BitmapBrushTip };
export type CircleBrushTip = { graph: Graph };
export type SquareBrushTip = { graph: Graph };
export type BitmapBrushTip = { width: number; height: number; data: Uint8Array };
