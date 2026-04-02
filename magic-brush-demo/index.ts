import { App } from "./loader.ts";

const canvas = document.getElementById("canvas") as HTMLCanvasElement;
const app = await App.create(canvas);
app.configure(canvas.width, canvas.height);

canvas.addEventListener("pointerdown", e => app.penDown(e));
canvas.addEventListener("pointermove", e => app.penMove(e));
canvas.addEventListener("pointerup", e => app.penUp(e));

console.log(app);