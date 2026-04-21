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

/// Apply mask dilation by iteratively expanding the boundary (6-connectivity).
pub fn dilate_mask(mask: &[u8], nx: usize, ny: usize, nz: usize, iterations: usize) -> Vec<u8> {
    let mut current = mask.to_vec();
    for _ in 0..iterations {
        let mut dilated = current.clone();
        for z in 0..nz {
            for y in 0..ny {
                for x in 0..nx {
                    let idx = x + y * nx + z * nx * ny;
                    if current[idx] == 1 {
                        continue;
                    }
                    // Set to 1 if any 6-connectivity neighbor is 1
                    let has_neighbor = (x > 0 && current[idx - 1] == 1)
                        || (x < nx - 1 && current[idx + 1] == 1)
                        || (y > 0 && current[idx - nx] == 1)
                        || (y < ny - 1 && current[idx + nx] == 1)
                        || (z > 0 && current[idx - nx * ny] == 1)
                        || (z < nz - 1 && current[idx + nx * ny] == 1);
                    if has_neighbor {
                        dilated[idx] = 1;
                    }
                }
            }
        }
        current = dilated;
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

/// Compute the obliquity angle (in degrees) from a NIfTI affine matrix.
///
/// Returns the angle between the scanner z-axis and the closest cardinal axis
/// in voxel space. A perfectly axial acquisition returns 0.
pub fn obliquity_from_affine(affine: &[f64; 16]) -> f64 {
    // Extract 3x3 rotation/scaling
    let cols: [[f64; 3]; 3] = [
        [affine[0], affine[4], affine[8]],
        [affine[1], affine[5], affine[9]],
        [affine[2], affine[6], affine[10]],
    ];

    // Find the maximum absolute value in each column to determine the
    // "dominant axis". The obliquity is the worst-case angle from cardinal.
    let mut max_obliquity: f64 = 0.0;
    for col in &cols {
        let norm = (col[0] * col[0] + col[1] * col[1] + col[2] * col[2]).sqrt();
        if norm < 1e-10 {
            continue;
        }
        // For each column, find the component with the largest absolute value
        let max_component = col.iter().map(|v| v.abs()).fold(0.0f64, f64::max);
        // cos(angle) = max_component / norm
        let cos_angle = (max_component / norm).min(1.0);
        let angle_deg = cos_angle.acos().to_degrees();
        if angle_deg > max_obliquity {
            max_obliquity = angle_deg;
        }
    }
    max_obliquity
}

/// Trilinear interpolation at a floating-point voxel coordinate.
fn trilinear_sample(data: &[f64], nx: usize, ny: usize, nz: usize, x: f64, y: f64, z: f64) -> f64 {
    let x0 = (x.floor() as isize).max(0).min(nx as isize - 1) as usize;
    let y0 = (y.floor() as isize).max(0).min(ny as isize - 1) as usize;
    let z0 = (z.floor() as isize).max(0).min(nz as isize - 1) as usize;
    let x1 = (x0 + 1).min(nx - 1);
    let y1 = (y0 + 1).min(ny - 1);
    let z1 = (z0 + 1).min(nz - 1);

    let fx = x - x0 as f64;
    let fy = y - y0 as f64;
    let fz = z - z0 as f64;

    let idx = |x: usize, y: usize, z: usize| x + y * nx + z * nx * ny;

    let c000 = data[idx(x0, y0, z0)];
    let c100 = data[idx(x1, y0, z0)];
    let c010 = data[idx(x0, y1, z0)];
    let c110 = data[idx(x1, y1, z0)];
    let c001 = data[idx(x0, y0, z1)];
    let c101 = data[idx(x1, y0, z1)];
    let c011 = data[idx(x0, y1, z1)];
    let c111 = data[idx(x1, y1, z1)];

    c000 * (1.0 - fx) * (1.0 - fy) * (1.0 - fz)
        + c100 * fx * (1.0 - fy) * (1.0 - fz)
        + c010 * (1.0 - fx) * fy * (1.0 - fz)
        + c110 * fx * fy * (1.0 - fz)
        + c001 * (1.0 - fx) * (1.0 - fy) * fz
        + c101 * fx * (1.0 - fy) * fz
        + c011 * (1.0 - fx) * fy * fz
        + c111 * fx * fy * fz
}

/// Result of resampling a volume to axial orientation.
pub struct ResampledVolume {
    pub data: Vec<f64>,
    pub dims: (usize, usize, usize),
    pub voxel_size: (f64, f64, f64),
    pub affine: [f64; 16],
}

/// Resample a volume from oblique orientation to axial (cardinal-aligned).
///
/// Computes the bounding box in world coordinates, creates a new grid with
/// voxel axes aligned to scanner axes, and trilinear-interpolates the data.
/// The output affine is diagonal (cardinal-aligned) with the same voxel sizes.
pub fn resample_to_axial(
    data: &[f64],
    nx: usize, ny: usize, nz: usize,
    affine: &[f64; 16],
) -> ResampledVolume {
    // Extract rotation/scaling columns and translation
    let r = [
        [affine[0], affine[1], affine[2]],
        [affine[4], affine[5], affine[6]],
        [affine[8], affine[9], affine[10]],
    ];
    let t = [affine[3], affine[7], affine[11]];

    // Compute voxel sizes from column norms
    let vsx = (r[0][0] * r[0][0] + r[1][0] * r[1][0] + r[2][0] * r[2][0]).sqrt();
    let vsy = (r[0][1] * r[0][1] + r[1][1] * r[1][1] + r[2][1] * r[2][1]).sqrt();
    let vsz = (r[0][2] * r[0][2] + r[1][2] * r[1][2] + r[2][2] * r[2][2]).sqrt();

    // Find world-space bounding box by transforming all 8 corners
    let corners_vox: [(f64, f64, f64); 8] = [
        (0.0, 0.0, 0.0),
        (nx as f64 - 1.0, 0.0, 0.0),
        (0.0, ny as f64 - 1.0, 0.0),
        (0.0, 0.0, nz as f64 - 1.0),
        (nx as f64 - 1.0, ny as f64 - 1.0, 0.0),
        (nx as f64 - 1.0, 0.0, nz as f64 - 1.0),
        (0.0, ny as f64 - 1.0, nz as f64 - 1.0),
        (nx as f64 - 1.0, ny as f64 - 1.0, nz as f64 - 1.0),
    ];

    let mut world_min = [f64::INFINITY; 3];
    let mut world_max = [f64::NEG_INFINITY; 3];
    for &(vi, vj, vk) in &corners_vox {
        for d in 0..3 {
            let w = r[d][0] * vi + r[d][1] * vj + r[d][2] * vk + t[d];
            if w < world_min[d] { world_min[d] = w; }
            if w > world_max[d] { world_max[d] = w; }
        }
    }

    // New grid dimensions using original voxel sizes
    let new_nx = ((world_max[0] - world_min[0]) / vsx).ceil() as usize + 1;
    let new_ny = ((world_max[1] - world_min[1]) / vsy).ceil() as usize + 1;
    let new_nz = ((world_max[2] - world_min[2]) / vsz).ceil() as usize + 1;

    // New affine: diagonal (cardinal-aligned), translation = world_min
    let new_affine = [
        vsx, 0.0, 0.0, world_min[0],
        0.0, vsy, 0.0, world_min[1],
        0.0, 0.0, vsz, world_min[2],
        0.0, 0.0, 0.0, 1.0,
    ];

    // Compute inverse of original affine's 3x3 rotation for world→voxel mapping
    let det = r[0][0] * (r[1][1] * r[2][2] - r[1][2] * r[2][1])
        - r[0][1] * (r[1][0] * r[2][2] - r[1][2] * r[2][0])
        + r[0][2] * (r[1][0] * r[2][1] - r[1][1] * r[2][0]);

    let inv_det = 1.0 / det;
    let inv_r = [
        [
            (r[1][1] * r[2][2] - r[1][2] * r[2][1]) * inv_det,
            (r[0][2] * r[2][1] - r[0][1] * r[2][2]) * inv_det,
            (r[0][1] * r[1][2] - r[0][2] * r[1][1]) * inv_det,
        ],
        [
            (r[1][2] * r[2][0] - r[1][0] * r[2][2]) * inv_det,
            (r[0][0] * r[2][2] - r[0][2] * r[2][0]) * inv_det,
            (r[0][2] * r[1][0] - r[0][0] * r[1][2]) * inv_det,
        ],
        [
            (r[1][0] * r[2][1] - r[1][1] * r[2][0]) * inv_det,
            (r[0][1] * r[2][0] - r[0][0] * r[2][1]) * inv_det,
            (r[0][0] * r[1][1] - r[0][1] * r[1][0]) * inv_det,
        ],
    ];

    // Resample: for each new voxel, find its world coord, map to original voxel space
    let mut new_data = vec![0.0f64; new_nx * new_ny * new_nz];
    for nk in 0..new_nz {
        for nj in 0..new_ny {
            for ni in 0..new_nx {
                // World coordinate of new voxel
                let wx = world_min[0] + ni as f64 * vsx;
                let wy = world_min[1] + nj as f64 * vsy;
                let wz = world_min[2] + nk as f64 * vsz;

                // Map to original voxel space: inv_R * (world - translation)
                let dx = wx - t[0];
                let dy = wy - t[1];
                let dz = wz - t[2];
                let ox = inv_r[0][0] * dx + inv_r[0][1] * dy + inv_r[0][2] * dz;
                let oy = inv_r[1][0] * dx + inv_r[1][1] * dy + inv_r[1][2] * dz;
                let oz = inv_r[2][0] * dx + inv_r[2][1] * dy + inv_r[2][2] * dz;

                // Check bounds (with small margin for interpolation)
                if ox >= -0.5 && ox <= nx as f64 - 0.5
                    && oy >= -0.5 && oy <= ny as f64 - 0.5
                    && oz >= -0.5 && oz <= nz as f64 - 0.5
                {
                    new_data[ni + nj * new_nx + nk * new_nx * new_ny] =
                        trilinear_sample(data, nx, ny, nz, ox, oy, oz);
                }
            }
        }
    }

    ResampledVolume {
        data: new_data,
        dims: (new_nx, new_ny, new_nz),
        voxel_size: (vsx, vsy, vsz),
        affine: new_affine,
    }
}

/// Resample a binary mask to axial using nearest-neighbor interpolation.
pub fn resample_mask_to_axial(
    mask: &[u8],
    nx: usize, ny: usize, nz: usize,
    affine: &[f64; 16],
) -> Vec<u8> {
    let mask_f64: Vec<f64> = mask.iter().map(|&m| m as f64).collect();
    let resampled = resample_to_axial(&mask_f64, nx, ny, nz, affine);
    resampled.data.iter().map(|&v| if v > 0.5 { 1u8 } else { 0u8 }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    // --- scale_phase_to_pi ---

    #[test]
    fn test_scale_phase_empty_array() {
        let mut data: Vec<f64> = vec![];
        scale_phase_to_pi(&mut data);
        assert!(data.is_empty());
    }

    #[test]
    fn test_scale_phase_already_in_pi_range() {
        let mut data = vec![-PI, 0.0, PI];
        let original = data.clone();
        scale_phase_to_pi(&mut data);
        // Should be unchanged (within tolerance)
        for (a, b) in data.iter().zip(original.iter()) {
            assert!((a - b).abs() < 1e-10, "Data changed when already in range");
        }
    }

    #[test]
    fn test_scale_phase_rescales_0_to_4096() {
        let mut data = vec![0.0, 2048.0, 4096.0];
        scale_phase_to_pi(&mut data);
        assert!((data[0] - (-PI)).abs() < 1e-10, "Min should map to -PI");
        assert!((data[2] - PI).abs() < 1e-10, "Max should map to PI");
        assert!(data[1].abs() < 1e-10, "Midpoint should map to ~0");
    }

    #[test]
    fn test_scale_phase_nan_replaced_with_zero() {
        let mut data = vec![0.0, f64::NAN, 4096.0];
        scale_phase_to_pi(&mut data);
        // NaN was replaced with 0.0 before rescaling
        // 0.0 maps to -PI (it's the min of the finite values)
        assert!(data[1].is_finite(), "NaN should be replaced with finite value");
    }

    #[test]
    fn test_scale_phase_constant_value() {
        let mut data = vec![5.0, 5.0, 5.0];
        scale_phase_to_pi(&mut data);
        // Range < 1e-10, returns early without rescaling
        assert!((data[0] - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_scale_phase_all_nan() {
        let mut data = vec![f64::NAN, f64::NAN, f64::NAN];
        scale_phase_to_pi(&mut data);
        // All replaced with 0, range is 0, returns early
        for v in &data {
            assert!((v - 0.0).abs() < 1e-10);
        }
    }

    // --- b0_direction_from_affine ---

    #[test]
    fn test_b0_direction_identity_matrix() {
        let mut affine = [0.0f64; 16];
        affine[0] = 1.0;
        affine[5] = 1.0;
        affine[10] = 1.0;
        affine[15] = 1.0;
        let (bx, by, bz) = b0_direction_from_affine(&affine);
        assert!(bx.abs() < 1e-6, "bx should be ~0, got {}", bx);
        assert!(by.abs() < 1e-6, "by should be ~0, got {}", by);
        assert!((bz - 1.0).abs() < 1e-6, "bz should be ~1, got {}", bz);
    }

    #[test]
    fn test_b0_direction_singular_matrix() {
        let affine = [0.0f64; 16]; // All zeros, det=0
        let (bx, by, bz) = b0_direction_from_affine(&affine);
        assert!((bx - 0.0).abs() < 1e-10);
        assert!((by - 0.0).abs() < 1e-10);
        assert!((bz - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_b0_direction_scaled_matrix() {
        let mut affine = [0.0f64; 16];
        affine[0] = 2.0;
        affine[5] = 2.0;
        affine[10] = 2.0;
        affine[15] = 1.0;
        let (bx, by, bz) = b0_direction_from_affine(&affine);
        // Scaled identity, normalized result should still be (0, 0, 1)
        assert!(bx.abs() < 1e-6);
        assert!(by.abs() < 1e-6);
        assert!((bz - 1.0).abs() < 1e-6);
    }

    // --- hz_to_ppm / rads_to_ppm ---

    #[test]
    fn test_hz_to_ppm_3t() {
        let gamma_hz = 42.576e6;
        let field = vec![gamma_hz * 3.0]; // 1 ppm worth of Hz at 3T
        let ppm = hz_to_ppm(&field, 3.0);
        assert!((ppm[0] - 1e6).abs() < 1.0, "Expected ~1e6 ppm, got {}", ppm[0]);
    }

    #[test]
    fn test_hz_to_ppm_7t() {
        let gamma_hz = 42.576e6;
        let field = vec![gamma_hz * 7.0]; // 1 ppm worth of Hz at 7T
        let ppm = hz_to_ppm(&field, 7.0);
        assert!((ppm[0] - 1e6).abs() < 1.0, "Expected ~1e6 ppm, got {}", ppm[0]);
    }

    #[test]
    fn test_rads_to_ppm_3t() {
        let gamma_hz = 42.576e6;
        // 1 ppm at 3T in rad/s = 2*PI * gamma * B0 * 1e-6
        let rads = vec![2.0 * PI * gamma_hz * 3.0];
        let ppm = rads_to_ppm(&rads, 3.0);
        assert!((ppm[0] - 1e6).abs() < 1.0, "Expected ~1e6 ppm, got {}", ppm[0]);
    }

    // --- rss_combine ---

    #[test]
    fn test_rss_combine_empty() {
        let result = rss_combine(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_rss_combine_single_echo() {
        let mag = vec![3.0, 4.0];
        let result = rss_combine(&[&mag]);
        assert!((result[0] - 3.0).abs() < 1e-10);
        assert!((result[1] - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_rss_combine_two_echoes() {
        let a = vec![3.0, 0.0];
        let b = vec![4.0, 1.0];
        let result = rss_combine(&[&a, &b]);
        assert!((result[0] - 5.0).abs() < 1e-10, "sqrt(9+16)=5");
        assert!((result[1] - 1.0).abs() < 1e-10, "sqrt(0+1)=1");
    }

    // --- erode_mask ---

    #[test]
    fn test_erode_mask_zero_iterations() {
        let mask = vec![1u8; 27]; // 3x3x3 all ones
        let result = erode_mask(&mask, 3, 3, 3, 0);
        assert_eq!(result, mask);
    }

    #[test]
    fn test_erode_mask_one_iteration_cube() {
        let mask = vec![1u8; 27]; // 3x3x3 all ones
        let result = erode_mask(&mask, 3, 3, 3, 1);
        // Only center voxel (1,1,1) survives — all boundary voxels touch an edge
        let center = 1 + 1 * 3 + 1 * 9;
        for (i, &v) in result.iter().enumerate() {
            if i == center {
                assert_eq!(v, 1, "Center voxel should survive");
            } else {
                assert_eq!(v, 0, "Boundary voxel {} should be eroded", i);
            }
        }
    }

    #[test]
    fn test_erode_mask_empty_mask() {
        let mask = vec![0u8; 27];
        let result = erode_mask(&mask, 3, 3, 3, 5);
        assert_eq!(result, mask);
    }

    #[test]
    fn test_erode_mask_multiple_iterations() {
        // 5x5x5, 2 iterations should leave only the very center
        let mask = vec![1u8; 125];
        let result = erode_mask(&mask, 5, 5, 5, 2);
        let center = 2 + 2 * 5 + 2 * 25;
        assert_eq!(result[center], 1, "Center should survive 2 erosions");
        let total: u32 = result.iter().map(|&v| v as u32).sum();
        assert_eq!(total, 1, "Only center voxel should remain");
    }

    // --- dilate_mask ---

    #[test]
    fn test_dilate_mask_zero_iterations() {
        let mask = vec![0u8; 27]; // 3x3x3
        let result = dilate_mask(&mask, 3, 3, 3, 0);
        assert_eq!(result, mask);
    }

    #[test]
    fn test_dilate_mask_single_voxel() {
        // 3x3x3 with only center set
        let mut mask = vec![0u8; 27];
        mask[1 + 1 * 3 + 1 * 9] = 1; // center
        let result = dilate_mask(&mask, 3, 3, 3, 1);
        // Center + 6 face neighbors should be set
        let total: u32 = result.iter().map(|&v| v as u32).sum();
        assert_eq!(total, 7, "Center + 6 neighbors = 7 voxels");
    }

    #[test]
    fn test_dilate_erode_roundtrip() {
        // Start with 5x5x5 full, erode 1, dilate 1 — should recover most voxels
        // (not exact roundtrip because corners are lost, but center should be filled)
        let mask = vec![1u8; 125];
        let eroded = erode_mask(&mask, 5, 5, 5, 1);
        let restored = dilate_mask(&eroded, 5, 5, 5, 1);
        // The 3x3x3 inner core should be intact after erode+dilate
        let center = 2 + 2 * 5 + 2 * 25;
        assert_eq!(restored[center], 1);
        // Should have more voxels than eroded
        let eroded_count: u32 = eroded.iter().map(|&v| v as u32).sum();
        let restored_count: u32 = restored.iter().map(|&v| v as u32).sum();
        assert!(restored_count > eroded_count);
    }

    // --- mask_center_of_mass ---

    #[test]
    fn test_mask_center_of_mass_empty() {
        let mask = vec![0u8; 27]; // 3x3x3
        let (cx, cy, cz) = mask_center_of_mass(&mask, 3, 3, 3);
        assert_eq!((cx, cy, cz), (1, 1, 1), "Empty mask should return volume center");
    }

    #[test]
    fn test_mask_center_of_mass_single_voxel() {
        let mut mask = vec![0u8; 64]; // 4x4x4
        // Set voxel (2, 2, 2)
        mask[2 + 2 * 4 + 2 * 16] = 1;
        let (cx, cy, cz) = mask_center_of_mass(&mask, 4, 4, 4);
        assert_eq!((cx, cy, cz), (2, 2, 2));
    }

    #[test]
    fn test_mask_center_of_mass_symmetric() {
        let mut mask = vec![0u8; 27]; // 3x3x3
        // Set opposite corners: (0,0,0) and (2,2,2)
        mask[0] = 1;
        mask[2 + 2 * 3 + 2 * 9] = 1;
        let (cx, cy, cz) = mask_center_of_mass(&mask, 3, 3, 3);
        // CoM should be (1, 1, 1)
        assert_eq!((cx, cy, cz), (1, 1, 1));
    }

    // --- obliquity_from_affine ---

    #[test]
    fn test_obliquity_identity_is_zero() {
        let mut affine = [0.0f64; 16];
        affine[0] = 1.0;
        affine[5] = 1.0;
        affine[10] = 1.0;
        affine[15] = 1.0;
        let obliquity = obliquity_from_affine(&affine);
        assert!(obliquity < 0.01, "Identity should have ~0° obliquity, got {}", obliquity);
    }

    #[test]
    fn test_obliquity_scaled_identity_is_zero() {
        let mut affine = [0.0f64; 16];
        affine[0] = 2.0;
        affine[5] = 2.0;
        affine[10] = 2.0;
        affine[15] = 1.0;
        let obliquity = obliquity_from_affine(&affine);
        assert!(obliquity < 0.01, "Scaled identity should have ~0° obliquity, got {}", obliquity);
    }

    #[test]
    fn test_obliquity_rotated_is_nonzero() {
        // 45° rotation in XZ plane
        let angle = std::f64::consts::FRAC_PI_4;
        let c = angle.cos();
        let s = angle.sin();
        let mut affine = [0.0f64; 16];
        affine[0] = c;    // r00
        affine[2] = s;    // r02
        affine[5] = 1.0;  // r11
        affine[8] = -s;   // r20
        affine[10] = c;   // r22
        affine[15] = 1.0;
        let obliquity = obliquity_from_affine(&affine);
        assert!(obliquity > 40.0, "45° rotation should give ~45° obliquity, got {}", obliquity);
    }

    // --- trilinear_sample ---

    #[test]
    fn test_trilinear_at_grid_point() {
        // 2x2x2 volume with values 0..7
        let data: Vec<f64> = (0..8).map(|i| i as f64).collect();
        let val = trilinear_sample(&data, 2, 2, 2, 0.0, 0.0, 0.0);
        assert!((val - 0.0).abs() < 1e-10);
        let val = trilinear_sample(&data, 2, 2, 2, 1.0, 1.0, 1.0);
        assert!((val - 7.0).abs() < 1e-10);
    }

    #[test]
    fn test_trilinear_midpoint() {
        // 2x2x2 all zeros except (1,1,1)=8
        let mut data = vec![0.0f64; 8];
        data[1 + 1 * 2 + 1 * 4] = 8.0;
        // Midpoint (0.5, 0.5, 0.5) should be 8 * 0.5 * 0.5 * 0.5 = 1.0
        let val = trilinear_sample(&data, 2, 2, 2, 0.5, 0.5, 0.5);
        assert!((val - 1.0).abs() < 1e-10, "Expected 1.0, got {}", val);
    }

    // --- resample_to_axial ---

    #[test]
    fn test_resample_identity_affine_preserves_data() {
        // 3x3x3 volume with identity affine
        let data: Vec<f64> = (0..27).map(|i| i as f64).collect();
        let mut affine = [0.0f64; 16];
        affine[0] = 1.0;
        affine[5] = 1.0;
        affine[10] = 1.0;
        affine[15] = 1.0;

        let result = resample_to_axial(&data, 3, 3, 3, &affine);
        // Identity should produce same dimensions
        assert_eq!(result.dims, (3, 3, 3));
        // Values at integer grid points should match
        for (i, (&orig, &resampled)) in data.iter().zip(result.data.iter()).enumerate() {
            assert!(
                (orig - resampled).abs() < 1e-6,
                "Mismatch at voxel {}: {} vs {}",
                i, orig, resampled
            );
        }
    }

    #[test]
    fn test_resample_axial_affine_is_diagonal() {
        let mut affine = [0.0f64; 16];
        // Rotated affine
        let angle = 0.3_f64; // ~17 degrees
        affine[0] = angle.cos();
        affine[2] = angle.sin();
        affine[5] = 1.0;
        affine[8] = -angle.sin();
        affine[10] = angle.cos();
        affine[15] = 1.0;

        let data = vec![1.0f64; 27]; // 3x3x3
        let result = resample_to_axial(&data, 3, 3, 3, &affine);

        // Output affine should be diagonal (cardinal-aligned)
        assert!((result.affine[1]).abs() < 1e-10, "Off-diagonal should be 0");
        assert!((result.affine[2]).abs() < 1e-10, "Off-diagonal should be 0");
        assert!((result.affine[4]).abs() < 1e-10, "Off-diagonal should be 0");
        assert!(result.affine[0] > 0.0, "Diagonal should be positive voxel size");
    }

    // --- resample_mask_to_axial ---

    #[test]
    fn test_resample_mask_identity_preserves() {
        let mask = vec![0u8, 1, 0, 1, 1, 1, 0, 1, 0]; // 3x3x1
        let mut affine = [0.0f64; 16];
        affine[0] = 1.0;
        affine[5] = 1.0;
        affine[10] = 1.0;
        affine[15] = 1.0;

        let result = resample_mask_to_axial(&mask, 3, 3, 1, &affine);
        // With identity affine, mask should be preserved
        assert_eq!(result.len(), mask.len());
        assert_eq!(result, mask);
    }
}
