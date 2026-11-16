# QBNex showcase programs

The programs behind the pictures in the main [README](../../README.md#showcase--ตัวอย่างผลงาน).
Each is a single `.bas` file with no dependencies. Compile one and run it:

```bash
qb examples/showcase/starfighter.bas -o starfighter
```

| File | What it shows | Keys |
| --- | --- | --- |
| [`raytrace.bas`](raytrace.bas) | Ray tracer: spheres, shadows, reflections, anti-aliasing | any key closes |
| [`starfighter.bas`](starfighter.bas) | Playable shooter with sprites, particles and sound | Left/Right, Space, Esc |
| [`dashboard.bas`](dashboard.bas) | Sales dashboard: KPI cards, bar chart, trend line, donut | any key closes |
| [`textui.bas`](textui.bas) | DOS-style text application with windows and a dialog | any key closes |
| [`surface3d.bas`](surface3d.bas) | Shaded 3D surface plot of z = sin(r)/r | any key closes |
| [`mandelbrot.bas`](mandelbrot.bas) | Smooth-coloured Mandelbrot zoom | any key closes |
| [`fractal_tree.bas`](fractal_tree.bas) | Recursive fractal tree at sunset | any key closes |
| [`life.bas`](life.bas) | Conway's Game of Life, coloured by cell age | Space = new world, other keys close |
| [`physics3d.bas`](physics3d.bas) | Five physics scenes on the GPU: pendulum wave, orbits under gravity, colliding balls, wave interference, Lorenz attractor; no text on the picture (the scene name is in the window title) | 1-5/N = scene, A = tour, Esc |
| [`shapes3d.bas`](shapes3d.bas) | Six spinning 3D shapes (infinity, trefoil and torus knots, Moebius strip, Klein bottle, seashell) on the GPU (`_DRAW3D`) or with `_MAPTRIANGLE` 3D | 1-6/N = shape, M = renderer, A = tour, Esc |

ไฟล์ `.bas` ทุกไฟล์คอมไพล์และรันได้ทันทีด้วย `qb` ลองแก้ค่าในโค้ด (สี มุมกล้อง จำนวนวัตถุ) แล้วคอมไพล์ใหม่เพื่อดูผลได้เลย
