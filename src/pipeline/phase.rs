use std::f64::consts::PI;

/// Scale phase data to [-pi, pi] range in-place.
///
/// If data is already in [-pi, pi] (within 10% tolerance), leave as-is.
/// Otherwise linearly map from [min, max] to [-pi, pi].
pub fn scale_phase_to_pi(data: &mut [f64]) {
    if data.is_empty() {
        return;
    }

    let mut min_val = f64::INFINITY;
    let mut max_val = f64::NEG_INFINITY;
    for &v in data.iter() {
        if v.is_finite() {
            if v < min_val {
                min_val = v;
            }
            if v > max_val {
                max_val = v;
            }
        }
    }

    // Replace NaN with 0
    for v in data.iter_mut() {
        if !v.is_finite() {
            *v = 0.0;
        }
    }

    let range = max_val - min_val;
    if range < 1e-10 {
        return;
    }

    // Check if already approximately in [-pi, pi]
    let tol = 0.1 * PI;
    if (min_val + PI).abs() < tol && (max_val - PI).abs() < tol {
        return;
    }

    // Linearly rescale to [-pi, pi]
    let scale = 2.0 * PI / range;
    for v in data.iter_mut() {
        *v = (*v - min_val) * scale - PI;
    }
}

/// Compute B0 direction from NIfTI affine matrix.
///
/// B0 is assumed along z-axis in scanner coordinates [0, 0, 1].
/// Transform to voxel coordinates using inverse rotation matrix.
pub fn b0_direction_from_affine(affine: &[f64; 16]) -> (f64, f64, f64) {
    // Extract 3x3 rotation/scaling from affine
    let r00 = affine[0];
    let r01 = affine[1];
    let r02 = affine[2];
    let r10 = affine[4];
    let r11 = affine[5];
    let r12 = affine[6];
    let r20 = affine[8];
    let r21 = affine[9];
    let r22 = affine[10];

    // Compute inverse of 3x3 rotation matrix
    let det = r00 * (r11 * r22 - r12 * r21) - r01 * (r10 * r22 - r12 * r20)
        + r02 * (r10 * r21 - r11 * r20);

    if det.abs() < 1e-10 {
        return (0.0, 0.0, 1.0); // Fallback
    }

    let inv_det = 1.0 / det;

    // Inverse rotation applied to [0, 0, 1] (scanner z-axis)
    // Only need the third column of the inverse matrix
    let bx = (r01 * r12 - r02 * r11) * inv_det;
    let by = (r02 * r10 - r00 * r12) * inv_det;
    let bz = (r00 * r11 - r01 * r10) * inv_det;

    // Normalize
    let norm = (bx * bx + by * by + bz * bz).sqrt();
    if norm < 1e-10 {
        return (0.0, 0.0, 1.0);
    }

    (bx / norm, by / norm, bz / norm)
}

/// Convert field from Hz to ppm.
pub fn hz_to_ppm(field_hz: &[f64], b0_tesla: f64) -> Vec<f64> {
    let gamma_hz = 42.576e6; // Hz/T (gyromagnetic ratio for proton)
    let scale = 1e6 / (gamma_hz * b0_tesla);
    field_hz.iter().map(|&v| v * scale).collect()
}

/// Convert field from rad/s to ppm.
pub fn rads_to_ppm(field_rads: &[f64], b0_tesla: f64) -> Vec<f64> {
    let gamma_hz = 42.576e6;
    let scale = 1e6 / (2.0 * PI * gamma_hz * b0_tesla);
    field_rads.iter().map(|&v| v * scale).collect()
}

/// Root-sum-of-squares combination of multiple magnitude images.
pub fn rss_combine(magnitudes: &[&[f64]]) -> Vec<f64> {
    if magnitudes.is_empty() {
        return Vec::new();
    }
    let n = magnitudes[0].len();
    let mut combined = vec![0.0f64; n];
    for mag in magnitudes {
        for (i, &v) in mag.iter().enumerate() {
            combined[i] += v * v;
        }
    }
    for v in &mut combined {
        *v = v.sqrt();
    }
    combined
}

/// Apply mask erosion by iteratively eroding the boundary.
pub fn erode_mask(mask: &[u8], nx: usize, ny: usize, nz: usize, iterations: usize) -> Vec<u8> {
    let mut current = mask.to_vec();
    for _ in 0..iterations {
        let mut eroded = current.clone();
        for z in 0..nz {
            for y in 0..ny {
                for x in 0..nx {
                    let idx = x + y * nx + z * nx * ny;
                    if current[idx] == 0 {
                        continue;
                    }
                    // Check 6-connectivity neighbors
                    if x == 0
                        || x == nx - 1
                        || y == 0
                        || y == ny - 1
                        || z == 0
                        || z == nz - 1
                        || current[idx - 1] == 0
                        || current[idx + 1] == 0
                        || current[idx - nx] == 0
                        || current[idx + nx] == 0
                        || current[idx - nx * ny] == 0
                        || current[idx + nx * ny] == 0
                    {
                        eroded[idx] = 0;
                    }
                }
            }
        }
        current = eroded;
    }
    current
}

/// Find the center of mass of a binary mask (for ROMEO seed point).
pub fn mask_center_of_mass(mask: &[u8], nx: usize, ny: usize, nz: usize) -> (usize, usize, usize) {
    let mut sx = 0.0f64;
    let mut sy = 0.0f64;
    let mut sz = 0.0f64;
    let mut count = 0.0f64;

    for z in 0..nz {
        for y in 0..ny {
            for x in 0..nx {
                if mask[x + y * nx + z * nx * ny] > 0 {
                    sx += x as f64;
                    sy += y as f64;
                    sz += z as f64;
                    count += 1.0;
                }
            }
        }
    }

    if count < 1.0 {
        return (nx / 2, ny / 2, nz / 2);
    }

    (
        (sx / count) as usize,
        (sy / count) as usize,
        (sz / count) as usize,
    )
}
