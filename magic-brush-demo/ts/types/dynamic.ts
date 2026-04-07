import { Graph } from "./graph.ts";

export type Dynamic = { base: number; modifiers: Modifier[] };
export type Modifier = { sensor: Sensor; graph: Graph };
export type Sensor =
    | "pressure"
    | "azimuth"
    | "altitude"
    | "tiltX"
    | "tiltY"
    | "twist"
    | { distance: DistanceSensor }
    | { speed: SpeedSensor }
    | { time: TimeSensor }
    | "jitterStroke"
    | "jitterDab";
export type DistanceSensor = { max: number };
export type SpeedSensor = { max: number };
export type TimeSensor = { max: number };
