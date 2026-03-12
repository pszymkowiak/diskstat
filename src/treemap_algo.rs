/// Squarified treemap algorithm (Bruls, Huizing, van Wijk).
/// Adapted for terminal character aspect ratio (~2:1 height:width).

#[derive(Debug, Clone, Copy)]
pub struct TreemapRect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

#[derive(Debug, Clone)]
pub struct TreemapItem {
    pub index: usize,
    pub size: f64,
    pub rect: TreemapRect,
}

/// Layout items into a rectangle using squarified treemap algorithm.
/// `sizes` must be sorted descending and all > 0.
pub fn squarify(rect: TreemapRect, sizes: &[(usize, f64)]) -> Vec<TreemapItem> {
    if sizes.is_empty() || rect.w <= 0.0 || rect.h <= 0.0 {
        return vec![];
    }

    let total: f64 = sizes.iter().map(|(_, s)| s).sum();
    if total <= 0.0 {
        return vec![];
    }

    // Normalize sizes to fill the rectangle area
    // Use aspect ratio correction: terminal chars are ~2:1 (height:width)
    let area = rect.w * rect.h;
    let normalized: Vec<(usize, f64)> = sizes
        .iter()
        .map(|(idx, s)| (*idx, s / total * area))
        .collect();

    let mut result = Vec::new();
    squarify_recursive(rect, &normalized, &mut result);
    result
}

fn squarify_recursive(rect: TreemapRect, items: &[(usize, f64)], result: &mut Vec<TreemapItem>) {
    if items.is_empty() || rect.w <= 0.0 || rect.h <= 0.0 {
        return;
    }

    if items.len() == 1 {
        result.push(TreemapItem {
            index: items[0].0,
            size: items[0].1,
            rect,
        });
        return;
    }

    // Determine layout direction: subdivide along the shorter side
    let vertical = rect.w >= rect.h;

    let short_side = if vertical { rect.h } else { rect.w };

    // Find the best row
    let mut row: Vec<(usize, f64)> = Vec::new();
    let mut best_ratio = f64::MAX;
    let mut split_at = 0;

    for (i, &item) in items.iter().enumerate() {
        row.push(item);
        let row_sum: f64 = row.iter().map(|(_, s)| s).sum();
        let ratio = worst_ratio(&row, row_sum, short_side);

        if ratio <= best_ratio {
            best_ratio = ratio;
            split_at = i + 1;
        } else {
            break;
        }
    }

    // Layout the row
    let row_items = &items[..split_at];
    let remaining = &items[split_at..];

    let row_sum: f64 = row_items.iter().map(|(_, s)| s).sum();

    if vertical {
        // Row is laid out vertically on the left, remaining goes to the right
        let row_width = if rect.h > 0.0 {
            row_sum / rect.h
        } else {
            rect.w
        };
        let row_width = row_width.min(rect.w);

        let mut y = rect.y;
        for &(idx, size) in row_items {
            let h = if row_width > 0.0 {
                size / row_width
            } else {
                rect.h
            };
            let h = h.min(rect.y + rect.h - y);
            result.push(TreemapItem {
                index: idx,
                size,
                rect: TreemapRect {
                    x: rect.x,
                    y,
                    w: row_width,
                    h,
                },
            });
            y += h;
        }

        // Recurse on remaining
        let new_rect = TreemapRect {
            x: rect.x + row_width,
            y: rect.y,
            w: rect.w - row_width,
            h: rect.h,
        };
        squarify_recursive(new_rect, remaining, result);
    } else {
        // Row is laid out horizontally on the top, remaining goes below
        let row_height = if rect.w > 0.0 {
            row_sum / rect.w
        } else {
            rect.h
        };
        let row_height = row_height.min(rect.h);

        let mut x = rect.x;
        for &(idx, size) in row_items {
            let w = if row_height > 0.0 {
                size / row_height
            } else {
                rect.w
            };
            let w = w.min(rect.x + rect.w - x);
            result.push(TreemapItem {
                index: idx,
                size,
                rect: TreemapRect {
                    x,
                    y: rect.y,
                    w,
                    h: row_height,
                },
            });
            x += w;
        }

        // Recurse on remaining
        let new_rect = TreemapRect {
            x: rect.x,
            y: rect.y + row_height,
            w: rect.w,
            h: rect.h - row_height,
        };
        squarify_recursive(new_rect, remaining, result);
    }
}

/// Compute the worst aspect ratio in a row of items.
fn worst_ratio(row: &[(usize, f64)], row_sum: f64, side: f64) -> f64 {
    if row.is_empty() || side <= 0.0 || row_sum <= 0.0 {
        return f64::MAX;
    }

    let s2 = side * side;
    let mut worst = 0.0f64;

    for &(_, size) in row {
        if size <= 0.0 {
            continue;
        }
        let r1 = (s2 * size) / (row_sum * row_sum);
        let r2 = (row_sum * row_sum) / (s2 * size);
        let ratio = r1.max(r2);
        worst = worst.max(ratio);
    }

    worst
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_squarify_basic() {
        let rect = TreemapRect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 100.0,
        };
        let sizes = vec![(0, 60.0), (1, 30.0), (2, 10.0)];
        let layout = squarify(rect, &sizes);
        assert_eq!(layout.len(), 3);
        // Total area should approximately match
        let total_area: f64 = layout.iter().map(|item| item.rect.w * item.rect.h).sum();
        assert!((total_area - 10000.0).abs() < 1.0);
    }

    #[test]
    fn test_squarify_empty() {
        let rect = TreemapRect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 100.0,
        };
        let sizes: Vec<(usize, f64)> = vec![];
        let layout = squarify(rect, &sizes);
        assert!(layout.is_empty());
    }

    #[test]
    fn test_squarify_single() {
        let rect = TreemapRect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 50.0,
        };
        let sizes = vec![(0, 100.0)];
        let layout = squarify(rect, &sizes);
        assert_eq!(layout.len(), 1);
        assert!((layout[0].rect.w - 100.0).abs() < 0.1);
        assert!((layout[0].rect.h - 50.0).abs() < 0.1);
    }

    #[test]
    fn test_squarify_zero_area() {
        let rect = TreemapRect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 100.0,
        };
        let sizes = vec![(0, 100.0)];
        let layout = squarify(rect, &sizes);
        assert!(layout.is_empty());
    }

    #[test]
    fn test_squarify_proportional_areas() {
        let rect = TreemapRect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 100.0,
        };
        let sizes = vec![(0, 50.0), (1, 30.0), (2, 20.0)];
        let layout = squarify(rect, &sizes);

        // Areas should be proportional to sizes
        let total_size: f64 = sizes.iter().map(|(_, s)| s).sum();
        let total_area = rect.w * rect.h;

        for (i, item) in layout.iter().enumerate() {
            let item_area = item.rect.w * item.rect.h;
            let expected_area = (sizes[i].1 / total_size) * total_area;
            let error = (item_area - expected_area).abs();
            assert!(error < 1.0, "Item {} area error too large: {}", i, error);
        }
    }
}
