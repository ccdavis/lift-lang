#!/usr/bin/env python3
"""
Mandelbrot Set Visualizer in Python - Large version (recursive)
Resolution: 200x100, Iterations: 100
"""

def mandelbrot_iter(cx, cy, zx, zy, iter_count, max_iter):
    """Recursive mandelbrot checker"""
    if iter_count >= max_iter:
        return True  # In set
    else:
        zx2 = zx * zx
        zy2 = zy * zy
        if zx2 + zy2 > 4.0:
            return False  # Escaped
        else:
            new_zx = zx2 - zy2 + cx
            new_zy = 2.0 * zx * zy + cy
            return mandelbrot_iter(cx, cy, new_zx, new_zy, iter_count + 1, max_iter)

def in_set(cx, cy, max_iter):
    """Check if a point is in the Mandelbrot set"""
    return mandelbrot_iter(cx, cy, 0.0, 0.0, 0, max_iter)

def build_row(cx, cy, dx, count, max_iter, acc=""):
    """Build a row string recursively"""
    if count == 0:
        return acc
    else:
        char = '*' if in_set(cx, cy, max_iter) else '.'
        return build_row(cx + dx, cy, dx, count - 1, max_iter, acc + char)

def render_rows(cx_start, cy, dx, dy, width, max_iter, rows_left):
    """Render all rows recursively"""
    if rows_left == 0:
        return 0
    else:
        row = build_row(cx_start, cy, dx, width, max_iter, "")
        print(row)
        return render_rows(cx_start, cy + dy, dx, dy, width, max_iter, rows_left - 1)

def visualize():
    """Main visualization"""
    width = 200
    height = 100
    max_iter = 100

    x_min = -2.5
    x_max = 1.0
    y_min = -1.0
    y_max = 1.0

    dx = (x_max - x_min) / 200.0
    dy = (y_max - y_min) / 100.0

    return render_rows(cx_start=x_min, cy=y_max, dx=dx, dy=-dy,
                      width=width, max_iter=max_iter, rows_left=height)

if __name__ == "__main__":
    # Increase recursion limit for larger visualizations
    import sys
    sys.setrecursionlimit(50000)

    visualize()
