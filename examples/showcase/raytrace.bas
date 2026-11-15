' Ray tracer in plain BASIC: spheres, shadows, reflections, a checkered floor
' and 2x2 anti-aliasing. Hot loops run through QBNex's Cranelift JIT.
' Compile:  qb raytrace.bas   then run ./raytrace
DEFSNG A-Z
CONST SCRW = 640, SCRH = 400, NS = 4, DEPTH = 3
SCREEN _NEWIMAGE(SCRW, SCRH, 32)
_TITLE "QBNex ray tracer"

DIM SHARED px(NS - 1), py(NS - 1), pz(NS - 1), rad(NS - 1)
DIM SHARED cr(NS - 1), cg(NS - 1), cb(NS - 1)
DIM SHARED lx, ly, lz
FOR i = 0 TO NS - 1: READ px(i), py(i), pz(i), rad(i), cr(i), cg(i), cb(i): NEXT
DATA -1.25,0,5,1,235,70,70
DATA 0.95,-0.35,4.1,0.65,70,140,245
DATA 0.05,-0.72,3.1,0.28,250,205,70
DATA 2.7,0.25,7.2,1.25,80,215,130

lx = -0.55: ly = 0.75: lz = -0.4
n = SQR(lx * lx + ly * ly + lz * lz): lx = lx / n: ly = ly / n: lz = lz / n

FOR y = 0 TO SCRH - 1
    FOR x = 0 TO SCRW - 1
        tr = 0: tg = 0: tb = 0
        FOR sy = 0 TO 1
            FOR sx = 0 TO 1
                dx = (x + sx * .5 - SCRW / 2) / SCRH
                dy = -(y + sy * .5 - SCRH / 2) / SCRH - .07
                dz = 1
                n = SQR(dx * dx + dy * dy + 1)
                Trace 0, .35, 0, dx / n, dy / n, dz / n, r, g, b
                tr = tr + r: tg = tg + g: tb = tb + b
            NEXT
        NEXT
        PSET (x, y), _RGB32(tr / 4, tg / 4, tb / 4)
    NEXT
NEXT
SLEEP
SYSTEM

' Nearest hit along (ox,oy,oz)+t*(dx,dy,dz): sphere index, -1 for the floor, -2 for none.
FUNCTION Hit (ox, oy, oz, dx, dy, dz, best)
    Hit = -2: best = 1E+30
    FOR i = 0 TO NS - 1
        qx = ox - px(i): qy = oy - py(i): qz = oz - pz(i)
        b = qx * dx + qy * dy + qz * dz
        c = qx * qx + qy * qy + qz * qz - rad(i) * rad(i)
        disc = b * b - c
        IF disc > 0 THEN
            t = -b - SQR(disc)
            IF t > .001 AND t < best THEN best = t: Hit = i
        END IF
    NEXT
    IF dy < 0 THEN
        t = (-1 - oy) / dy
        IF t > .001 AND t < best THEN best = t: Hit = -1
    END IF
END FUNCTION

SUB Trace (ox, oy, oz, dx, dy, dz, r, g, b)
    r = 0: g = 0: b = 0: w = 1
    FOR bounce = 1 TO DEPTH
        k = Hit(ox, oy, oz, dx, dy, dz, t)
        IF k = -2 THEN ' sky
            s = dy: IF s < 0 THEN s = 0
            r = r + w * (250 - 190 * s): g = g + w * (235 - 110 * s): b = b + w * 255
            EXIT SUB
        END IF
        hx = ox + t * dx: hy = oy + t * dy: hz = oz + t * dz
        IF k = -1 THEN
            nx = 0: ny = 1: nz = 0: refl = .3
            IF (INT(hx) + INT(hz)) AND 1 THEN
                sr = 235: sg = 235: sb = 235
            ELSE
                sr = 40: sg = 45: sb = 60
            END IF
        ELSE
            nx = (hx - px(k)) / rad(k): ny = (hy - py(k)) / rad(k): nz = (hz - pz(k)) / rad(k)
            sr = cr(k): sg = cg(k): sb = cb(k): refl = .35
        END IF
        diff = nx * lx + ny * ly + nz * lz: IF diff < 0 THEN diff = 0
        IF Hit(hx, hy, hz, lx, ly, lz, ts) <> -2 THEN diff = diff * .25
        hhx = lx - dx: hhy = ly - dy: hhz = lz - dz
        n = SQR(hhx * hhx + hhy * hhy + hhz * hhz)
        spec = (nx * hhx + ny * hhy + nz * hhz) / n: IF spec < 0 OR diff < .3 THEN spec = 0
        spec = spec ^ 60 * 255
        lit = .12 + .88 * diff
        r = r + w * (1 - refl) * (sr * lit + spec)
        g = g + w * (1 - refl) * (sg * lit + spec)
        b = b + w * (1 - refl) * (sb * lit + spec)
        w = w * refl
        dd = 2 * (dx * nx + dy * ny + dz * nz)
        dx = dx - dd * nx: dy = dy - dd * ny: dz = dz - dd * nz
        ox = hx: oy = hy: oz = hz
    NEXT
END SUB
