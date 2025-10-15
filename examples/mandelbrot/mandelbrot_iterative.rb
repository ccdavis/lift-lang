#!/usr/bin/env ruby
# Mandelbrot Set Visualizer - Iterative version
# Ruby implementation matching the Lift version

def in_mandelbrot(cx, cy, max_iter)
  # Check if a point is in the Mandelbrot set
  zx = 0.0
  zy = 0.0
  iter_count = 0
  escaped = false

  while iter_count < max_iter && !escaped
    zx2 = zx * zx
    zy2 = zy * zy

    if zx2 + zy2 > 4.0
      # Escaped - not in set
      escaped = true
    else
      # Continue iterating
      new_zy = 2.0 * zx * zy + cy
      zx = zx2 - zy2 + cx
      zy = new_zy
      iter_count += 1
    end
  end

  # If we didn't escape, point is in set
  !escaped
end

def visualize
  # Main visualization function
  width = 60
  height = 30
  max_iter = 50

  # Mandelbrot set bounds
  x_min = -2.0
  x_max = 1.0
  y_min = -1.0
  y_max = 1.0

  dx = (x_max - x_min) / 60.0
  dy = (y_max - y_min) / 30.0

  # Iterate through each row
  row = 0
  while row < height
    # Calculate y coordinate (flip for correct orientation)
    y_flt = 1.0 * row
    cy = y_max - y_flt * dy

    # Build the row string
    line = ''
    col = 0
    while col < width
      # Calculate x coordinate
      x_flt = 1.0 * col
      cx = x_min + x_flt * dx

      # Check if point is in set
      if in_mandelbrot(cx, cy, max_iter)
        line += '*'
      else
        line += '.'
      end

      col += 1
    end

    # Output the completed row
    puts line
    row += 1
  end

  0
end

if __FILE__ == $0
  visualize
end
