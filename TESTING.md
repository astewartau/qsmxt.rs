# QSMxT.rs Manual Testing Guide

Test data: `/home/ashley/repos/qsm/bids/` — single subject (sub-1), 4-echo MEGRE at 7T, 164x205x205 @ 1mm isotropic, with forward-simulated derivatives (Chimap, mask, fieldmaps, segmentation).

All commands assume you're in the qsmxt.rs directory and have built with `cargo build --release`. Use `./target/release/qsmxt` or alias it.

```bash
alias qsmxt=./target/release/qsmxt
BIDS=/home/ashley/repos/qsm/bids
OUT=/tmp/qsmxt-test
MAG=$BIDS/sub-1/anat/sub-1_echo-1_part-mag_MEGRE.nii
PHA=$BIDS/sub-1/anat/sub-1_echo-1_part-phase_MEGRE.nii
PHA2=$BIDS/sub-1/anat/sub-1_echo-2_part-phase_MEGRE.nii
MAG2=$BIDS/sub-1/anat/sub-1_echo-2_part-mag_MEGRE.nii
MAG3=$BIDS/sub-1/anat/sub-1_echo-3_part-mag_MEGRE.nii
MAG4=$BIDS/sub-1/anat/sub-1_echo-4_part-mag_MEGRE.nii
MASK=$BIDS/derivatives/qsm-forward/sub-1/anat/sub-1_mask.nii
CHIMAP=$BIDS/derivatives/qsm-forward/sub-1/anat/sub-1_Chimap.nii
LOCALFIELD=$BIDS/derivatives/qsm-forward/sub-1/anat/sub-1_fieldmap-local.nii
FIELDMAP=$BIDS/derivatives/qsm-forward/sub-1/anat/sub-1_fieldmap.nii
rm -rf $OUT && mkdir -p $OUT
```

---

## 1. Pipeline Meta Commands

### 1.1 Validate BIDS dataset
```bash
qsmxt validate $BIDS
```
**Expected:** Lists sub-1 with 4 echoes, 7T, magnitude available. No errors.

### 1.2 List presets
```bash
qsmxt presets
```
**Expected:** Lists gre, epi, bet, fast, body with descriptions.

### 1.3 Show preset details
```bash
qsmxt presets gre
```
**Expected:** Full TOML config for GRE preset (RTS, ROMEO, PDF, threshold masking, erosions=[2]).

### 1.4 Generate config file
```bash
qsmxt init --preset gre --output $OUT/gre.toml
```
**Expected:** `$OUT/gre.toml` exists, contains `qsm_algorithm = "rts"`, `bf_algorithm = "pdf"`.

### 1.5 Dry run
```bash
qsmxt run $BIDS $OUT/dry --dry
```
**Expected:** Prints pipeline info, sub-1 with 4 echoes, 164x205x205, B0=7.0T, memory estimate. No files created in `$OUT/dry`.

---

## 2. Standalone Masking Commands

### 2.1 BET brain extraction
```bash
qsmxt bet $MAG -o $OUT/bet_mask.nii
```
**Expected:** `$OUT/bet_mask.nii` — binary mask, nonzero voxels ~30-60% of volume.

### 2.2 BET with custom fractional intensity
```bash
qsmxt bet $MAG -o $OUT/bet_loose.nii --fractional-intensity 0.3
```
**Expected:** `$OUT/bet_loose.nii` — larger brain region than default (0.5).

### 2.3 Threshold mask (Otsu)
```bash
qsmxt mask $MAG -o $OUT/otsu_mask.nii
```
**Expected:** `$OUT/otsu_mask.nii` — binary mask from Otsu thresholding on magnitude.

### 2.4 Threshold mask with erosions
```bash
qsmxt mask $MAG -o $OUT/eroded_mask.nii --erosions 3
```
**Expected:** `$OUT/eroded_mask.nii` — smaller mask than 2.3 (3 erosion iterations).

### 2.5 Dilate mask
```bash
qsmxt dilate $MASK -o $OUT/dilated_mask.nii --iterations 2
```
**Expected:** `$OUT/dilated_mask.nii` — larger than input mask.

### 2.6 Morphological close
```bash
qsmxt close $MASK -o $OUT/closed_mask.nii --radius 2
```
**Expected:** `$OUT/closed_mask.nii` — holes/gaps in boundary filled.

### 2.7 Fill holes
```bash
qsmxt fill-holes $MASK -o $OUT/filled_mask.nii --max-size 5000
```
**Expected:** `$OUT/filled_mask.nii` — internal holes up to 5000 voxels filled. Reports voxels filled count.

### 2.8 Gaussian smooth mask
```bash
qsmxt smooth-mask $MASK -o $OUT/smoothed_mask.nii --sigma 3.0
```
**Expected:** `$OUT/smoothed_mask.nii` — smoothed and re-thresholded at 0.5. Slightly rounder boundaries.

### 2.9 Chain mask operations (script)
```bash
qsmxt mask $MAG -o $OUT/chain_step1.nii
qsmxt dilate $OUT/chain_step1.nii -o $OUT/chain_step2.nii --iterations 1
qsmxt fill-holes $OUT/chain_step2.nii -o $OUT/chain_step3.nii --max-size 2000
qsmxt close $OUT/chain_step3.nii -o $OUT/chain_final.nii --radius 1
```
**Expected:** Each step produces a file; final mask should be cleaner than step 1.

---

## 3. Standalone Phase/QSM Commands

### 3.1 Phase unwrapping (Laplacian)
```bash
qsmxt unwrap $PHA --mask $MASK -o $OUT/unwrapped_lap.nii --algorithm laplacian
```
**Expected:** `$OUT/unwrapped_lap.nii` — unwrapped phase, values should be smooth within mask (no 2pi jumps).

### 3.2 Phase unwrapping (ROMEO)
```bash
qsmxt unwrap $PHA --mask $MASK -o $OUT/unwrapped_romeo.nii --algorithm romeo --magnitude $MAG
```
**Expected:** `$OUT/unwrapped_romeo.nii` — unwrapped phase, should be similar to Laplacian but potentially better at boundaries.

### 3.3 Background field removal (V-SHARP)
```bash
qsmxt bgremove $FIELDMAP --mask $MASK -o $OUT/local_vsharp.nii --algorithm vsharp
```
**Expected:** `$OUT/local_vsharp.nii` — local field map with background removed.

### 3.4 Background field removal (PDF)
```bash
qsmxt bgremove $FIELDMAP --mask $MASK -o $OUT/local_pdf.nii --algorithm pdf
```
**Expected:** `$OUT/local_pdf.nii` — local field.

### 3.5 Background field removal with output mask
```bash
qsmxt bgremove $FIELDMAP --mask $MASK -o $OUT/local_lbv.nii --algorithm lbv --output-mask $OUT/lbv_mask.nii
```
**Expected:** Both `$OUT/local_lbv.nii` and `$OUT/lbv_mask.nii` created.

### 3.6 Dipole inversion (RTS)
```bash
qsmxt invert $LOCALFIELD --mask $MASK -o $OUT/chi_rts.nii --algorithm rts
```
**Expected:** `$OUT/chi_rts.nii` — susceptibility map. Values typically in range [-0.2, 0.2] ppm.

### 3.7 Dipole inversion (TKD)
```bash
qsmxt invert $LOCALFIELD --mask $MASK -o $OUT/chi_tkd.nii --algorithm tkd
```
**Expected:** `$OUT/chi_tkd.nii` — faster than RTS, potentially more streaking artifacts.

### 3.8 Dipole inversion (TV)
```bash
qsmxt invert $LOCALFIELD --mask $MASK -o $OUT/chi_tv.nii --algorithm tv
```
**Expected:** `$OUT/chi_tv.nii` — TV-regularized, smoother than TKD.

### 3.9 SWI
```bash
qsmxt swi $PHA $MAG --mask $MASK -o $OUT/swi.nii --mip --mip-output $OUT/mip.nii
```
**Expected:** `$OUT/swi.nii` + `$OUT/mip.nii` — SWI and minimum intensity projection.

---

## 4. Standalone Quantitative Mapping Commands

### 4.1 R2* mapping
```bash
qsmxt r2star $MAG $MAG2 $MAG3 $MAG4 \
  --mask $MASK -o $OUT/r2star.nii \
  --echo-times 0.004 0.012 0.020 0.028
```
**Expected:** `$OUT/r2star.nii` — R2* values in Hz. Typical brain tissue: 20-80 Hz at 7T. Zero outside mask.

### 4.2 T2* mapping
```bash
qsmxt t2star $MAG $MAG2 $MAG3 $MAG4 \
  --mask $MASK -o $OUT/t2star.nii \
  --echo-times 0.004 0.012 0.020 0.028
```
**Expected:** `$OUT/t2star.nii` — T2* values in seconds. Typical brain tissue: 0.01-0.05s at 7T. Zero outside mask.

### 4.3 Inhomogeneity correction
```bash
qsmxt homogeneity $MAG -o $OUT/mag_corrected.nii
```
**Expected:** `$OUT/mag_corrected.nii` — magnitude with more uniform intensity. Spatial bias reduced.

### 4.4 Inhomogeneity correction with custom sigma
```bash
qsmxt homogeneity $MAG -o $OUT/mag_corrected_wide.nii --sigma 15.0
```
**Expected:** `$OUT/mag_corrected_wide.nii` — wider smoothing kernel, corrects larger-scale inhomogeneity.

### 4.5 Phase quality map
```bash
qsmxt quality-map $PHA -o $OUT/quality.nii --magnitude $MAG
```
**Expected:** `$OUT/quality.nii` — values 0-100 per voxel. High values where phase is coherent.

### 4.6 Phase quality map (two echoes)
```bash
qsmxt quality-map $PHA -o $OUT/quality_2echo.nii \
  --magnitude $MAG --phase2 $PHA2 --te1 0.004 --te2 0.012
```
**Expected:** `$OUT/quality_2echo.nii` — improved quality estimate using dual-echo information.

### 4.7 Resample to axial
```bash
qsmxt resample $MAG -o $OUT/resampled.nii
```
**Expected:** `$OUT/resampled.nii` — for this data (already axial), dimensions should be similar. Affine should be diagonal.

---

## 5. Full QSM Pipeline

### 5.1 Default pipeline (GRE preset)
```bash
qsmxt run $BIDS $OUT/pipeline-gre
```
**Expected files:**
- `$OUT/pipeline-gre/dataset_description.json`
- `$OUT/pipeline-gre/pipeline_config.toml`
- `$OUT/pipeline-gre/sub-1/anat/sub-1_Chimap.nii` — QSM map
- `$OUT/pipeline-gre/sub-1/anat/sub-1_mask.nii` — brain mask

### 5.2 Pipeline with SWI + T2* + R2*
```bash
qsmxt run $BIDS $OUT/pipeline-full --do-swi --do-t2starmap --do-r2starmap
```
**Expected files:**
- `$OUT/pipeline-full/sub-1/anat/sub-1_Chimap.nii`
- `$OUT/pipeline-full/sub-1/anat/sub-1_mask.nii`
- `$OUT/pipeline-full/sub-1/anat/sub-1_swi.nii`
- `$OUT/pipeline-full/sub-1/anat/sub-1_minIP.nii`
- `$OUT/pipeline-full/sub-1/anat/sub-1_T2starmap.nii`
- `$OUT/pipeline-full/sub-1/anat/sub-1_R2starmap.nii`

### 5.3 Pipeline with TGV algorithm (Body preset)
```bash
qsmxt run $BIDS $OUT/pipeline-tgv --preset body
```
**Expected files:**
- `$OUT/pipeline-tgv/sub-1/anat/sub-1_Chimap.nii` — TGV single-step QSM
- `$OUT/pipeline-tgv/sub-1/anat/sub-1_mask.nii`
- `$OUT/pipeline-tgv/pipeline_config.toml` — should show `qsm_algorithm = "tgv"`

### 5.4 Pipeline with TKD algorithm
```bash
qsmxt run $BIDS $OUT/pipeline-tkd --qsm-algorithm tkd
```
**Expected:** Same output structure as 5.1. TKD is fastest, may have more streaking.

### 5.5 Pipeline with TV algorithm
```bash
qsmxt run $BIDS $OUT/pipeline-tv --qsm-algorithm tv
```
**Expected:** Same output structure. TV should be smoother than TKD.

### 5.6 Pipeline with BET masking
```bash
qsmxt run $BIDS $OUT/pipeline-bet --preset bet
```
**Expected:** Same outputs. Mask generated via BET on magnitude. May differ from threshold mask.

### 5.7 Pipeline with V-SHARP background removal
```bash
qsmxt run $BIDS $OUT/pipeline-vsharp --bf-algorithm vsharp
```
**Expected:** Same outputs. V-SHARP erodes the mask (output mask should be smaller).

### 5.8 Pipeline with Laplacian unwrapping
```bash
qsmxt run $BIDS $OUT/pipeline-lap --unwrapping-algorithm laplacian
```
**Expected:** Same outputs. Different unwrapping may produce subtle differences in QSM.

### 5.9 Pipeline with phase-quality masking
```bash
qsmxt run $BIDS $OUT/pipeline-pq --masking-input phase-quality
```
**Expected:** Same output structure. Mask based on ROMEO quality map instead of phase magnitude.

### 5.10 Pipeline with inhomogeneity correction
```bash
qsmxt run $BIDS $OUT/pipeline-inhomog --inhomogeneity-correction
```
**Expected:** Same output structure. Magnitude corrected before masking, may produce cleaner mask.

### 5.11 Pipeline with custom mask operations
```bash
qsmxt run $BIDS $OUT/pipeline-maskops \
  --mask-op input:magnitude \
  --mask-op threshold:otsu \
  --mask-op erode:1 \
  --mask-op close:1 \
  --mask-op fill-holes:2000 \
  --mask-op dilate:1
```
**Expected:** Same output structure. Mask built via custom sequence instead of legacy masking.

### 5.12 Pipeline with config file
```bash
qsmxt init --preset fast --output $OUT/fast.toml
qsmxt run $BIDS $OUT/pipeline-config --config $OUT/fast.toml
```
**Expected:** Pipeline uses Fast preset settings from TOML file.

### 5.13 Pipeline limiting echoes
```bash
qsmxt run $BIDS $OUT/pipeline-2echo --num-echoes 2
```
**Expected:** Same output structure, but only uses first 2 echoes for processing.

### 5.14 Pipeline with debug logging
```bash
qsmxt run $BIDS $OUT/pipeline-debug --debug 2>&1 | head -50
```
**Expected:** Verbose debug output including algorithm parameters, timing, memory estimates.

### 5.15 SLURM script generation
```bash
qsmxt slurm $BIDS $OUT/slurm-test --account testacct --partition gpu --time 01:00:00
```
**Expected files:**
- `$OUT/slurm-test/slurm/qsmxt_sub-1.sh` — SLURM batch script
- `$OUT/slurm-test/pipeline_config.toml`
- Script contains `#SBATCH --account=testacct`, `#SBATCH --partition=gpu`

---

## 6. Edge Cases and Error Handling

### 6.1 Missing BIDS directory
```bash
qsmxt run /nonexistent $OUT/fail 2>&1
```
**Expected:** Error message about missing directory.

### 6.2 Invalid algorithm
```bash
qsmxt run $BIDS $OUT/fail --qsm-algorithm tgv --bf-algorithm vsharp 2>&1
```
**Expected:** Should work (TGV ignores BF algorithm, logs a debug message about it).

### 6.3 BET without magnitude
```bash
qsmxt bet $PHA -o $OUT/fail.nii 2>&1
```
**Expected:** Runs BET on phase data (may produce poor mask since BET expects magnitude).

### 6.4 R2* with too few echoes
```bash
qsmxt r2star $MAG $MAG2 --mask $MASK -o $OUT/fail.nii --echo-times 0.004 0.012 2>&1
```
**Expected:** Error — requires 3+ echoes.

### 6.5 Invalid mask-op
```bash
qsmxt run $BIDS $OUT/fail --mask-op "foobar:123" 2>&1
```
**Expected:** Warning about invalid mask-op, falls back to defaults.

---

## 7. TUI Tests

Launch the TUI:
```bash
qsmxt tui
```

### 7.1 Navigation
- [ ] Tab 1-5 keys switch between tabs (Input/Output, Filters, Algorithms, Parameters, Execution)
- [ ] Arrow keys navigate between fields within a tab
- [ ] Tab / Shift+Tab cycle between tabs

### 7.2 Input/Output tab
- [ ] Type BIDS directory path in "BIDS Dir" field
- [ ] Type output directory path in "Output Dir" field
- [ ] Cycle through presets (None, GRE, EPI, BET, Fast, Body) with Enter/Space
- [ ] Type a config file path

### 7.3 Filters tab
- [ ] Enter subject IDs (e.g., "1")
- [ ] Enter session, acquisition, run filters
- [ ] Enter num_echoes limit

### 7.4 Algorithms tab
- [ ] Cycle QSM algorithm: RTS → TV → TKD → TGV
- [ ] Cycle unwrapping: ROMEO → Laplacian
- [ ] Cycle BG removal: V-SHARP → PDF → LBV → iSMV
- [ ] Cycle masking algorithm: BET → Threshold
- [ ] Cycle masking input: Phase → Magnitude → Phase-Quality

### 7.5 Parameters tab
- [ ] Toggle "Combine Phase" checkbox
- [ ] Edit BET fractional intensity
- [ ] Edit mask erosions (e.g., "2 3")
- [ ] Edit RTS/TGV/TV/TKD parameters
- [ ] Edit obliquity threshold (e.g., "5.0")

### 7.6 Execution tab
- [ ] Toggle "Compute SWI" checkbox
- [ ] Toggle "Compute T2* Map" checkbox
- [ ] Toggle "Compute R2* Map" checkbox
- [ ] Toggle "Inhomogeneity Correction" checkbox
- [ ] Toggle "Dry Run" checkbox
- [ ] Toggle "Debug Logging" checkbox
- [ ] Edit "Num Processes"

### 7.7 Command preview
- [ ] Verify the command preview at the bottom updates as you change fields
- [ ] Changing QSM algorithm updates `--qsm-algorithm` in preview
- [ ] Toggling SWI adds `--do-swi` to preview
- [ ] Setting obliquity threshold adds `--obliquity-threshold` to preview
- [ ] Masking input "phase-quality" shows `--masking-input phase-quality`

### 7.8 Execute from TUI
- [ ] Fill in BIDS dir and output dir
- [ ] Press F5 to execute
- [ ] Verify pipeline runs and output files are created
- [ ] Verify TUI restores terminal properly after execution

### 7.9 Quit
- [ ] Press `q` or `Esc` to quit
- [ ] Terminal is restored to normal state (no raw mode artifacts)

---

## 8. Comparison with Forward-Simulated Ground Truth

The derivatives in `$BIDS/derivatives/qsm-forward/` contain ground truth data. After running the pipeline, you can visually compare:

```bash
# Compare QSM output with ground truth
# (Use a NIfTI viewer like fsleyes, mrview, or ITK-SNAP)
# Ground truth: $BIDS/derivatives/qsm-forward/sub-1/anat/sub-1_Chimap.nii
# Pipeline:     $OUT/pipeline-gre/sub-1/anat/sub-1_Chimap.nii

# Compare masks
# Ground truth: $BIDS/derivatives/qsm-forward/sub-1/anat/sub-1_mask.nii
# Pipeline:     $OUT/pipeline-gre/sub-1/anat/sub-1_mask.nii
```

**What to look for:**
- QSM map should show similar susceptibility contrast to ground truth
- Mask should cover similar brain region
- No large artifacts or empty regions within the brain
- SWI should show venous structures as dark
- T2*/R2* maps should show reasonable tissue contrast

---

## 9. Automated Unit Tests

```bash
cargo test
```
**Expected:** 102 tests pass, 0 failures.
