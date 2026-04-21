#!/bin/bash
set -euo pipefail

# QSMxT.rs CLI Test Runner
# Run all CLI tests from TESTING.md sequentially.
# Each test gets its own numbered folder with stdout/stderr logs.
# Walk through TESTING.md and compare expected output against results/.

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
QSMXT="$SCRIPT_DIR/target/release/qsmxt"
BIDS=/home/ashley/repos/qsm/bids
RESULTS="${1:-/tmp/qsmxt-test-results}"

# Input file shortcuts
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

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
NC='\033[0m'

PASS_COUNT=0
FAIL_COUNT=0
SKIP_COUNT=0
TOTAL_COUNT=0

# Build first
echo -e "${CYAN}Building release binary...${NC}"
cargo build --release 2>/dev/null
if [ ! -f "$QSMXT" ]; then
    echo -e "${RED}Build failed. Cannot find $QSMXT${NC}"
    exit 1
fi

# Clean results
rm -rf "$RESULTS"
mkdir -p "$RESULTS"

run_test() {
    local num="$1"
    local name="$2"
    shift 2
    # Remaining args: the command parts (without qsmxt prefix)

    TOTAL_COUNT=$((TOTAL_COUNT + 1))
    local dir="$RESULTS/${num}_${name}"
    mkdir -p "$dir"

    local label="${num} ${name}"
    printf "  %-55s" "$label"

    local start_time=$(date +%s%N)

    # Run the command, capture stdout+stderr, and exit code
    set +e
    "$QSMXT" "$@" >"$dir/stdout.log" 2>"$dir/stderr.log"
    local exit_code=$?
    set -e

    local end_time=$(date +%s%N)
    local elapsed_ms=$(( (end_time - start_time) / 1000000 ))

    echo "$*" > "$dir/command.txt"
    echo "$exit_code" > "$dir/exit_code.txt"

    if [ $exit_code -eq 0 ]; then
        echo -e "${GREEN}PASS${NC}  (${elapsed_ms}ms)"
        PASS_COUNT=$((PASS_COUNT + 1))
    else
        echo -e "${RED}FAIL${NC}  (exit=$exit_code, ${elapsed_ms}ms)"
        FAIL_COUNT=$((FAIL_COUNT + 1))
    fi
}

run_test_expect_fail() {
    local num="$1"
    local name="$2"
    shift 2

    TOTAL_COUNT=$((TOTAL_COUNT + 1))
    local dir="$RESULTS/${num}_${name}"
    mkdir -p "$dir"

    local label="${num} ${name}"
    printf "  %-55s" "$label"

    set +e
    "$QSMXT" "$@" >"$dir/stdout.log" 2>"$dir/stderr.log"
    local exit_code=$?
    set -e

    echo "$*" > "$dir/command.txt"
    echo "$exit_code" > "$dir/exit_code.txt"

    if [ $exit_code -ne 0 ]; then
        echo -e "${GREEN}PASS${NC}  (expected failure, exit=$exit_code)"
        PASS_COUNT=$((PASS_COUNT + 1))
    else
        echo -e "${RED}FAIL${NC}  (expected failure but got exit=0)"
        FAIL_COUNT=$((FAIL_COUNT + 1))
    fi
}

# Move output files into the test result folder
collect_output() {
    local dir="$1"
    shift
    for f in "$@"; do
        if [ -e "$f" ]; then
            cp "$f" "$dir/" 2>/dev/null || true
        fi
    done
}

collect_tree() {
    local dir="$1"
    local src="$2"
    if [ -d "$src" ]; then
        cp -r "$src" "$dir/output_tree" 2>/dev/null || true
        find "$src" -type f | sort > "$dir/file_list.txt"
    fi
}

echo ""
echo -e "${CYAN}═══════════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}  QSMxT.rs CLI Test Suite${NC}"
echo -e "${CYAN}  Results: $RESULTS${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════════${NC}"

# ─────────────────────────────────────────────────────────
echo ""
echo -e "${YELLOW}Section 1: Pipeline Meta Commands${NC}"
# ─────────────────────────────────────────────────────────

run_test "1.1" "validate-bids" validate "$BIDS"

run_test "1.2" "list-presets" presets

run_test "1.3" "show-preset-gre" presets gre

OUT_1_4="$RESULTS/1.4_generate-config"
run_test "1.4" "generate-config" init --preset gre --output "$OUT_1_4/gre.toml"

OUT_1_5="$RESULTS/1.5_dry-run/dry_output"
run_test "1.5" "dry-run" run "$BIDS" "$OUT_1_5" --dry

# ─────────────────────────────────────────────────────────
echo ""
echo -e "${YELLOW}Section 2: Standalone Masking Commands${NC}"
# ─────────────────────────────────────────────────────────

D="$RESULTS/2.1_bet-extraction"
run_test "2.1" "bet-extraction" bet "$MAG" -o "$D/bet_mask.nii"

D="$RESULTS/2.2_bet-custom-fi"
run_test "2.2" "bet-custom-fi" bet "$MAG" -o "$D/bet_loose.nii" --fractional-intensity 0.3

D="$RESULTS/2.3_threshold-otsu"
run_test "2.3" "threshold-otsu" mask "$MAG" -o "$D/otsu_mask.nii"

D="$RESULTS/2.4_threshold-eroded"
run_test "2.4" "threshold-eroded" mask "$MAG" -o "$D/eroded_mask.nii" --erosions 3

D="$RESULTS/2.5_dilate-mask"
run_test "2.5" "dilate-mask" dilate "$MASK" -o "$D/dilated_mask.nii" --iterations 2

D="$RESULTS/2.6_morphological-close"
run_test "2.6" "morphological-close" close "$MASK" -o "$D/closed_mask.nii" --radius 2

D="$RESULTS/2.7_fill-holes"
run_test "2.7" "fill-holes" fill-holes "$MASK" -o "$D/filled_mask.nii" --max-size 5000

D="$RESULTS/2.8_gaussian-smooth"
run_test "2.8" "gaussian-smooth" smooth-mask "$MASK" -o "$D/smoothed_mask.nii" --sigma 3.0

# 2.9 Chain: multiple sequential commands
D="$RESULTS/2.9_chain-mask-ops"
mkdir -p "$D"
printf "  %-55s" "2.9 chain-mask-ops"
TOTAL_COUNT=$((TOTAL_COUNT + 1))
set +e
(
    "$QSMXT" mask "$MAG" -o "$D/step1_threshold.nii" &&
    "$QSMXT" dilate "$D/step1_threshold.nii" -o "$D/step2_dilate.nii" --iterations 1 &&
    "$QSMXT" fill-holes "$D/step2_dilate.nii" -o "$D/step3_fillholes.nii" --max-size 2000 &&
    "$QSMXT" close "$D/step3_fillholes.nii" -o "$D/step4_close.nii" --radius 1
) >"$D/stdout.log" 2>"$D/stderr.log"
chain_exit=$?
set -e
echo "mask + dilate + fill-holes + close" > "$D/command.txt"
echo "$chain_exit" > "$D/exit_code.txt"
if [ $chain_exit -eq 0 ]; then
    echo -e "${GREEN}PASS${NC}"
    PASS_COUNT=$((PASS_COUNT + 1))
else
    echo -e "${RED}FAIL${NC}  (exit=$chain_exit)"
    FAIL_COUNT=$((FAIL_COUNT + 1))
fi

# ─────────────────────────────────────────────────────────
echo ""
echo -e "${YELLOW}Section 3: Standalone Phase/QSM Commands${NC}"
# ─────────────────────────────────────────────────────────

D="$RESULTS/3.1_unwrap-laplacian"
run_test "3.1" "unwrap-laplacian" unwrap "$PHA" --mask "$MASK" -o "$D/unwrapped_lap.nii" --algorithm laplacian

D="$RESULTS/3.2_unwrap-romeo"
run_test "3.2" "unwrap-romeo" unwrap "$PHA" --mask "$MASK" -o "$D/unwrapped_romeo.nii" --algorithm romeo --magnitude "$MAG"

D="$RESULTS/3.3_bgremove-vsharp"
run_test "3.3" "bgremove-vsharp" bgremove "$FIELDMAP" --mask "$MASK" -o "$D/local_vsharp.nii" --algorithm vsharp

D="$RESULTS/3.4_bgremove-pdf"
run_test "3.4" "bgremove-pdf" bgremove "$FIELDMAP" --mask "$MASK" -o "$D/local_pdf.nii" --algorithm pdf

D="$RESULTS/3.5_bgremove-lbv"
run_test "3.5" "bgremove-lbv" bgremove "$FIELDMAP" --mask "$MASK" -o "$D/local_lbv.nii" --algorithm lbv --output-mask "$D/lbv_mask.nii"

D="$RESULTS/3.6_invert-rts"
run_test "3.6" "invert-rts" invert "$LOCALFIELD" --mask "$MASK" -o "$D/chi_rts.nii" --algorithm rts

D="$RESULTS/3.7_invert-tkd"
run_test "3.7" "invert-tkd" invert "$LOCALFIELD" --mask "$MASK" -o "$D/chi_tkd.nii" --algorithm tkd

D="$RESULTS/3.8_invert-tv"
run_test "3.8" "invert-tv" invert "$LOCALFIELD" --mask "$MASK" -o "$D/chi_tv.nii" --algorithm tv

D="$RESULTS/3.9_swi"
run_test "3.9" "swi" swi "$PHA" "$MAG" --mask "$MASK" -o "$D/swi.nii" --mip --mip-output "$D/mip.nii"

# ─────────────────────────────────────────────────────────
echo ""
echo -e "${YELLOW}Section 4: Standalone Quantitative Mapping Commands${NC}"
# ─────────────────────────────────────────────────────────

D="$RESULTS/4.1_r2star-mapping"
run_test "4.1" "r2star-mapping" r2star "$MAG" "$MAG2" "$MAG3" "$MAG4" --mask "$MASK" -o "$D/r2star.nii" --echo-times 0.004 0.012 0.020 0.028

D="$RESULTS/4.2_t2star-mapping"
run_test "4.2" "t2star-mapping" t2star "$MAG" "$MAG2" "$MAG3" "$MAG4" --mask "$MASK" -o "$D/t2star.nii" --echo-times 0.004 0.012 0.020 0.028

D="$RESULTS/4.3_inhomogeneity-correction"
run_test "4.3" "inhomogeneity-correction" homogeneity "$MAG" -o "$D/mag_corrected.nii"

D="$RESULTS/4.4_inhomogeneity-wide-sigma"
run_test "4.4" "inhomogeneity-wide-sigma" homogeneity "$MAG" -o "$D/mag_corrected_wide.nii" --sigma 15.0

D="$RESULTS/4.5_quality-map"
run_test "4.5" "quality-map" quality-map "$PHA" -o "$D/quality.nii" --magnitude "$MAG"

D="$RESULTS/4.6_quality-map-2echo"
run_test "4.6" "quality-map-2echo" quality-map "$PHA" -o "$D/quality_2echo.nii" --magnitude "$MAG" --phase2 "$PHA2" --te1 0.004 --te2 0.012

D="$RESULTS/4.7_resample-axial"
run_test "4.7" "resample-axial" resample "$MAG" -o "$D/resampled.nii"

# ─────────────────────────────────────────────────────────
echo ""
echo -e "${YELLOW}Section 5: Full QSM Pipeline${NC}"
# ─────────────────────────────────────────────────────────

D="$RESULTS/5.1_pipeline-gre"
run_test "5.1" "pipeline-gre" run "$BIDS" "$D/output"
collect_tree "$D" "$D/output"

D="$RESULTS/5.2_pipeline-full"
run_test "5.2" "pipeline-full" run "$BIDS" "$D/output" --do-swi --do-t2starmap --do-r2starmap
collect_tree "$D" "$D/output"

D="$RESULTS/5.3_pipeline-tgv"
run_test "5.3" "pipeline-tgv" run "$BIDS" "$D/output" --preset body
collect_tree "$D" "$D/output"

D="$RESULTS/5.4_pipeline-tkd"
run_test "5.4" "pipeline-tkd" run "$BIDS" "$D/output" --qsm-algorithm tkd
collect_tree "$D" "$D/output"

D="$RESULTS/5.5_pipeline-tv"
run_test "5.5" "pipeline-tv" run "$BIDS" "$D/output" --qsm-algorithm tv
collect_tree "$D" "$D/output"

D="$RESULTS/5.6_pipeline-bet"
run_test "5.6" "pipeline-bet" run "$BIDS" "$D/output" --preset bet
collect_tree "$D" "$D/output"

D="$RESULTS/5.7_pipeline-vsharp"
run_test "5.7" "pipeline-vsharp" run "$BIDS" "$D/output" --bf-algorithm vsharp
collect_tree "$D" "$D/output"

D="$RESULTS/5.8_pipeline-laplacian"
run_test "5.8" "pipeline-laplacian" run "$BIDS" "$D/output" --unwrapping-algorithm laplacian
collect_tree "$D" "$D/output"

D="$RESULTS/5.9_pipeline-phase-quality"
run_test "5.9" "pipeline-phase-quality" run "$BIDS" "$D/output" --masking-input phase-quality
collect_tree "$D" "$D/output"

D="$RESULTS/5.10_pipeline-inhomog"
run_test "5.10" "pipeline-inhomog" run "$BIDS" "$D/output" --inhomogeneity-correction
collect_tree "$D" "$D/output"

D="$RESULTS/5.11_pipeline-maskops"
run_test "5.11" "pipeline-maskops" run "$BIDS" "$D/output" \
    --mask-op input:magnitude \
    --mask-op threshold:otsu \
    --mask-op erode:1 \
    --mask-op close:1 \
    --mask-op fill-holes:2000 \
    --mask-op dilate:1
collect_tree "$D" "$D/output"

# 5.12: Two-step (init config, then run with it)
D="$RESULTS/5.12_pipeline-config"
mkdir -p "$D"
printf "  %-55s" "5.12 pipeline-config"
TOTAL_COUNT=$((TOTAL_COUNT + 1))
set +e
(
    "$QSMXT" init --preset fast --output "$D/fast.toml" &&
    "$QSMXT" run "$BIDS" "$D/output" --config "$D/fast.toml"
) >"$D/stdout.log" 2>"$D/stderr.log"
cfg_exit=$?
set -e
echo "init --preset fast + run --config" > "$D/command.txt"
echo "$cfg_exit" > "$D/exit_code.txt"
collect_tree "$D" "$D/output"
if [ $cfg_exit -eq 0 ]; then
    echo -e "${GREEN}PASS${NC}"
    PASS_COUNT=$((PASS_COUNT + 1))
else
    echo -e "${RED}FAIL${NC}  (exit=$cfg_exit)"
    FAIL_COUNT=$((FAIL_COUNT + 1))
fi

D="$RESULTS/5.13_pipeline-2echo"
run_test "5.13" "pipeline-2echo" run "$BIDS" "$D/output" --num-echoes 2
collect_tree "$D" "$D/output"

D="$RESULTS/5.14_pipeline-debug"
run_test "5.14" "pipeline-debug" run "$BIDS" "$D/output" --debug
collect_tree "$D" "$D/output"

D="$RESULTS/5.15_slurm-generation"
run_test "5.15" "slurm-generation" slurm "$BIDS" "$D/output" --account testacct --partition gpu --time 01:00:00
collect_tree "$D" "$D/output"

# ─────────────────────────────────────────────────────────
echo ""
echo -e "${YELLOW}Section 6: Edge Cases and Error Handling${NC}"
# ─────────────────────────────────────────────────────────

D="$RESULTS/6.1_missing-bids-dir"
run_test_expect_fail "6.1" "missing-bids-dir" run /nonexistent "$D/output"

D="$RESULTS/6.2_tgv-with-bf-override"
run_test "6.2" "tgv-with-bf-override" run "$BIDS" "$D/output" --qsm-algorithm tgv --bf-algorithm vsharp
collect_tree "$D" "$D/output"

D="$RESULTS/6.3_bet-on-phase"
run_test "6.3" "bet-on-phase" bet "$PHA" -o "$D/bet_phase.nii"

D="$RESULTS/6.4_r2star-too-few-echoes"
run_test_expect_fail "6.4" "r2star-too-few-echoes" r2star "$MAG" "$MAG2" --mask "$MASK" -o "$D/fail.nii" --echo-times 0.004 0.012

# 6.5: Invalid mask-op (may warn but not necessarily fail the whole pipeline)
D="$RESULTS/6.5_invalid-mask-op"
run_test "6.5" "invalid-mask-op" run "$BIDS" "$D/output" --mask-op "foobar:123"
collect_tree "$D" "$D/output"

# ─────────────────────────────────────────────────────────
echo ""
echo -e "${CYAN}═══════════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}  Results Summary${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "  ${GREEN}PASS: $PASS_COUNT${NC}"
echo -e "  ${RED}FAIL: $FAIL_COUNT${NC}"
echo -e "  Total: $TOTAL_COUNT"
echo ""
echo "  Results directory: $RESULTS"
echo "  Each test folder contains:"
echo "    command.txt   — the command that was run"
echo "    stdout.log    — standard output"
echo "    stderr.log    — standard error"
echo "    exit_code.txt — process exit code"
echo "    *.nii         — output NIfTI files (where applicable)"
echo "    file_list.txt — list of output files (for pipeline tests)"
echo ""

if [ $FAIL_COUNT -gt 0 ]; then
    echo -e "${RED}Some tests failed. Check stderr.log in failed test folders.${NC}"
    echo ""
    echo "Failed tests:"
    for d in "$RESULTS"/*/; do
        if [ -f "$d/exit_code.txt" ]; then
            code=$(cat "$d/exit_code.txt")
            name=$(basename "$d")
            # Check if this was an expect-fail test
            if echo "$name" | grep -qE "^6\.[14]_"; then
                # Expect-fail tests: fail means exit=0
                if [ "$code" = "0" ]; then
                    echo "  $name (expected failure but got exit=0)"
                fi
            else
                if [ "$code" != "0" ]; then
                    echo "  $name (exit=$code)"
                fi
            fi
        fi
    done
    echo ""
    exit 1
else
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
fi
