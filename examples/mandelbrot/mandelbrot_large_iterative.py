#!/usr/bin/env python3
"""
Mandelbrot Set Visualizer in Python - Large version (iterative)
Resolution: 200x100, Iterations: 100
"""

def in_mandelbrot(cx, cy, max_iter=100):
    """Check if a point is in the Mandelbrot set using iteration"""
    zx = 0.0
    zy = 0.0
    iter_count = 0

    while iter_count < max_iter:
        zx2 = zx * zx
        zy2 = zy * zy

        if zx2 + zy2 > 4.0:
            # Escaped - not in set
            return False

        # Continue iterating: z = z^2 + c
        new_zy = 2.0 * zx * zy + cy
        zx = zx2 - zy2 + cx
        zy = new_zy
        iter_count += 1

    # Completed all iterations without escaping - in set
    return True

def visualize():
    """Main visualization function"""
    width = 200
    height = 100
    max_iter = 100

    # Mandelbrot set bounds
    x_min = -2.5
    x_max = 1.0
    y_min = -1.0
    y_max = 1.0

    dx = (x_max - x_min) / width
    dy = (y_max - y_min) / height

    # Iterate through each row
    for row in range(height):
        # Calculate y coordinate (flip for correct orientation)
        cy = y_max - row * dy

        # Build the row string
        line = ""
        for col in range(width):
            # Calculate x coordinate
            cx = x_min + col * dx

            # Check if point is in set
            if in_mandelbrot(cx, cy, max_iter):
                line += '*'
            else:
                line += '.'

        # Output the completed row
        print(line)

if __name__ == "__main__":
    visualize()
