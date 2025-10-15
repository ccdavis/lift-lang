#!/usr/bin/env python3
"""
Mandelbrot Set Visualizer in Python
Matches the Lift implementation exactly
"""

max_iter = 50

def mandelbrot_iter(cx, cy, zx, zy, iter_count):
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
            return mandelbrot_iter(cx, cy, new_zx, new_zy, iter_count + 1)

def in_set(cx, cy):
    """Check if a point is in the Mandelbrot set"""
    return mandelbrot_iter(cx, cy, 0.0, 0.0, 0)

def build_row(cx, cy, dx, count, acc=""):
    """Build a row string recursively"""
    if count == 0:
        return acc
    else:
        char = '*' if in_set(cx, cy) else '.'
        return build_row(cx + dx, cy, dx, count - 1, acc + char)

def render_rows(cx_start, cy, dx, dy, width, rows_left):
    """Render all rows recursively"""
    if rows_left == 0:
        return 0
    else:
        row = build_row(cx_start, cy, dx, width, "")
        print(row)
        return render_rows(cx_start, cy + dy, dx, dy, width, rows_left - 1)

def visualize():
    """Main visualization"""
    width = 60
    height = 30

    x_min = -2.0
    x_max = 1.0
    y_min = -1.0
    y_max = 1.0

    dx = (x_max - x_min) / 60.0
    dy = (y_max - y_min) / 30.0

    return render_rows(cx_start=x_min, cy=y_max, dx=dx, dy=-dy,
                      width=width, rows_left=height)

if __name__ == "__main__":
    # Increase recursion limit for larger visualizations
    import sys
    sys.setrecursionlimit(10000)

    visualize()
