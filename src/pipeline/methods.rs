use super::config::*;

struct Citation {
    key: &'static str,
    text: &'static str,
}

const CITE_MCPC3DS: Citation = Citation {
    key: "eckstein2018",
    text: "Eckstein, K., et al. (2018). \"Computationally Efficient Combination of Multi-channel Phase Data From Multi-echo Acquisitions (ASPIRE).\" *Magnetic Resonance in Medicine*, 79:2996-3006. https://doi.org/10.1002/mrm.26963",
};

const CITE_ROMEO: Citation = Citation {
    key: "dymerska2021",
    text: "Dymerska, B., et al. (2021). \"Phase unwrapping with a rapid opensource minimum spanning tree algorithm (ROMEO).\" *Magnetic Resonance in Medicine*, 85(4):2294-2308. https://doi.org/10.1002/mrm.28563",
};

const CITE_LAPLACIAN_UNWRAP: Citation = Citation {
    key: "schofield2003",
    text: "Schofield, M.A., Zhu, Y. (2003). \"Fast phase unwrapping algorithm for interferometric applications.\" *Optics Letters*, 28(14):1194-1196. https://doi.org/10.1364/OL.28.001194",
};

const CITE_VSHARP: Citation = Citation {
    key: "wu2012",
    text: "Wu, B., et al. (2012). \"Whole brain susceptibility mapping using compressed sensing.\" *Magnetic Resonance in Medicine*, 67(1):137-147. https://doi.org/10.1002/mrm.23000",
};

const CITE_SHARP: Citation = Citation {
    key: "schweser2011",
    text: "Schweser, F., et al. (2011). \"Quantitative imaging of intrinsic magnetic tissue properties using MRI signal phase.\" *NeuroImage*, 54(4):2789-2807. https://doi.org/10.1016/j.neuroimage.2010.10.070",
};

const CITE_PDF: Citation = Citation {
    key: "liu2011pdf",
    text: "Liu, T., et al. (2011). \"A novel background field removal method for MRI using projection onto dipole fields.\" *NMR in Biomedicine*, 24(9):1129-1136. https://doi.org/10.1002/nbm.1670",
};

const CITE_ISMV: Citation = Citation {
    key: "wen2014",
    text: "Wen, Y., et al. (2014). \"An iterative spherical mean value method for background field removal in MRI.\" *Magnetic Resonance in Medicine*, 72(4):1065-1071. https://doi.org/10.1002/mrm.24998",
};

const CITE_LBV: Citation = Citation {
    key: "zhou2014",
    text: "Zhou, D., et al. (2014). \"Background field removal by solving the Laplacian boundary value problem.\" *NMR in Biomedicine*, 27(3):312-319. https://doi.org/10.1002/nbm.3064",
};

const CITE_RTS: Citation = Citation {
    key: "kames2018",
    text: "Kames, C., Wiggermann, V., Rauscher, A. (2018). \"Rapid two-step dipole inversion for susceptibility mapping with sparsity priors.\" *NeuroImage*, 167:276-283. https://doi.org/10.1016/j.neuroimage.2017.11.018",
};

const CITE_TV: Citation = Citation {
    key: "bilgic2014tv",
    text: "Bilgic, B., et al. (2014). \"Fast quantitative susceptibility mapping with L1-regularization and automatic parameter selection.\" *Magnetic Resonance in Medicine*, 72(5):1444-1459. https://doi.org/10.1002/mrm.25029",
};

const CITE_TKD: Citation = Citation {
    key: "shmueli2009",
    text: "Shmueli, K., et al. (2009). \"Magnetic susceptibility mapping of brain tissue in vivo using MRI phase data.\" *Magnetic Resonance in Medicine*, 62(6):1510-1522. https://doi.org/10.1002/mrm.22135",
};

const CITE_TIKHONOV: Citation = Citation {
    key: "bilgic2014l2",
    text: "Bilgic, B., et al. (2014). \"Fast image reconstruction with L2-regularization.\" *Journal of Magnetic Resonance Imaging*, 40(1):181-191. https://doi.org/10.1002/jmri.24365",
};

const CITE_NLTV: Citation = Citation {
    key: "kames2018",
    text: "Kames, C., Wiggermann, V., Rauscher, A. (2018). \"Rapid two-step dipole inversion for susceptibility mapping with sparsity priors.\" *NeuroImage*, 167:276-283. https://doi.org/10.1016/j.neuroimage.2017.11.018",
};

const CITE_MEDI: Citation = Citation {
    key: "liu2011medi",
    text: "Liu, T., et al. (2011). \"Morphology enabled dipole inversion (MEDI) from a single-angle acquisition.\" *Magnetic Resonance in Medicine*, 66(3):777-783. https://doi.org/10.1002/mrm.22816",
};

const CITE_ILSQR: Citation = Citation {
    key: "li2015",
    text: "Li, W., et al. (2015). \"A method for estimating and removing streaking artifacts in quantitative susceptibility mapping.\" *NeuroImage*, 108:111-122. https://doi.org/10.1016/j.neuroimage.2014.12.043",
};

const CITE_TGV: Citation = Citation {
    key: "langkammer2015",
    text: "Langkammer, C., et al. (2015). \"Fast quantitative susceptibility mapping using 3D EPI and total generalized variation.\" *NeuroImage*, 111:622-630. https://doi.org/10.1016/j.neuroimage.2015.02.041",
};

const CITE_QSMART: Citation = Citation {
    key: "yaghmaie2021",
    text: "Yaghmaie, N., Syeda, W., et al. (2021). \"QSMART: Quantitative Susceptibility Mapping Artifact Reduction Technique.\" *NeuroImage*, 231:117701. https://doi.org/10.1016/j.neuroimage.2020.117701",
};

const CITE_CLEARSWI: Citation = Citation {
    key: "eckstein2024",
    text: "Eckstein, K., et al. (2024). \"CLEAR-SWI: Computational Efficient T2* Weighted Imaging.\" *Proc. ISMRM*.",
};

const CITE_ARLO: Citation = Citation {
    key: "pei2015",
    text: "Pei, M., et al. (2015). \"Algorithm for fast monoexponential fitting based on Auto-Regression on Linear Operations (ARLO) of data.\" *Magnetic Resonance in Medicine*, 73(2):843-850. https://doi.org/10.1002/mrm.25137",
};

const CITE_OTSU: Citation = Citation {
    key: "otsu1979",
    text: "Otsu, N. (1979). \"A Threshold Selection Method from Gray-Level Histograms.\" *IEEE Transactions on Systems, Man, and Cybernetics*, 9(1):62-66. https://doi.org/10.1109/TSMC.1979.4310076",
};

const CITE_BET: Citation = Citation {
    key: "smith2002",
    text: "Smith, S.M. (2002). \"Fast robust automated brain extraction.\" *Human Brain Mapping*, 17(3):143-155. https://doi.org/10.1002/hbm.10062",
};

const CITE_BIAS: Citation = Citation {
    key: "eckstein2019",
    text: "Eckstein, K., Trattnig, S., Robinson, S.D. (2019). \"A Simple Homogeneity Correction for Neuroimaging at 7T.\" *Proc. ISMRM 27th Annual Meeting*.",
};

const CITE_QSMXT: Citation = Citation {
    key: "stewart2026",
    text: "Stewart, A. (2026). QSMxT.rs. https://github.com/astewartau/qsmxt.rs",
};

/// Generate a methods description and citation list from a pipeline configuration.
pub fn generate_methods(config: &PipelineConfig) -> String {
    let mut sentences = Vec::new();
    let mut citations: Vec<&Citation> = Vec::new();

    if config.do_qsm {
        sentences.push("QSM processing was performed using qsmxt.rs (Stewart, 2026).".to_string());
    } else {
        sentences.push("MRI processing was performed using qsmxt.rs (Stewart, 2026).".to_string());
    }
    add_citation(&mut citations, &CITE_QSMXT);

    // Masking (inhomogeneity correction is described inline)
    describe_masking(config, &mut sentences, &mut citations);

    // QSM-specific pipeline
    if config.do_qsm {
        match config.qsm_algorithm {
            QsmAlgorithm::Tgv => {
                sentences.push("QSM was computed using the Total Generalized Variation (TGV) algorithm (Langkammer et al., 2015), which performs phase unwrapping, background field removal, and dipole inversion in a single step.".to_string());
                add_citation(&mut citations, &CITE_TGV);
            }
            QsmAlgorithm::Qsmart => {
                sentences.push("QSM was computed using QSMART (Yaghmaie et al., 2021), a two-stage approach using spatially dependent filtering for background field removal, TKD inversion, and Frangi vesselness-based tissue/vasculature separation.".to_string());
                add_citation(&mut citations, &CITE_QSMART);
            }
            _ => {
                // Multi-echo combination
                if config.combine_phase {
                    sentences.push("Multi-echo phase data was combined using the MCPC-3D-S method (Eckstein et al., 2018).".to_string());
                    add_citation(&mut citations, &CITE_MCPC3DS);
                }

                // Unwrapping
                if let Some(ref alg) = config.unwrapping_algorithm {
                    match alg {
                        UnwrappingAlgorithm::Romeo => {
                            sentences.push("Phase unwrapping was performed using ROMEO (Dymerska et al., 2021).".to_string());
                            add_citation(&mut citations, &CITE_ROMEO);
                        }
                        UnwrappingAlgorithm::Laplacian => {
                            sentences.push("Phase unwrapping was performed using the Laplacian method (Schofield & Zhu, 2003).".to_string());
                            add_citation(&mut citations, &CITE_LAPLACIAN_UNWRAP);
                        }
                    }
                }

                // Background removal
                if let Some(ref alg) = config.bf_algorithm {
                    let (name, cite) = match alg {
                        BfAlgorithm::Vsharp => ("V-SHARP", &CITE_VSHARP),
                        BfAlgorithm::Pdf => ("PDF", &CITE_PDF),
                        BfAlgorithm::Lbv => ("LBV", &CITE_LBV),
                        BfAlgorithm::Ismv => ("iSMV", &CITE_ISMV),
                        BfAlgorithm::Sharp => ("SHARP", &CITE_SHARP),
                    };
                    sentences.push(format!("Background field removal was performed using {} ({}).", name, cite_inline(cite)));
                    add_citation(&mut citations, cite);
                }

                // Dipole inversion
                let (name, cite) = match config.qsm_algorithm {
                    QsmAlgorithm::Rts => ("RTS (Rapid Two-Step)", &CITE_RTS),
                    QsmAlgorithm::Tv => ("Total Variation (TV-ADMM)", &CITE_TV),
                    QsmAlgorithm::Tkd => ("TKD (Thresholded K-space Division)", &CITE_TKD),
                    QsmAlgorithm::Tsvd => ("TSVD (Truncated Singular Value Decomposition)", &CITE_TKD),
                    QsmAlgorithm::Tikhonov => ("Tikhonov regularization", &CITE_TIKHONOV),
                    QsmAlgorithm::Nltv => ("NLTV (Nonlinear Total Variation)", &CITE_NLTV),
                    QsmAlgorithm::Medi => ("MEDI (Morphology Enabled Dipole Inversion)", &CITE_MEDI),
                    QsmAlgorithm::Ilsqr => ("iLSQR", &CITE_ILSQR),
                    QsmAlgorithm::Tgv | QsmAlgorithm::Qsmart => unreachable!(),
                };
                sentences.push(format!("Dipole inversion was performed using {} ({}).", name, cite_inline(cite)));
                add_citation(&mut citations, cite);
            }
        }

        // Referencing
        match config.qsm_reference {
            QsmReference::Mean => {
                sentences.push("The resulting susceptibility map was mean-referenced within the brain mask.".to_string());
            }
            QsmReference::None => {
                sentences.push("No susceptibility referencing was applied.".to_string());
            }
        }
    }

    // SWI
    if config.do_swi {
        sentences.push("Susceptibility-weighted images were computed using CLEAR-SWI (Eckstein et al., 2024).".to_string());
        add_citation(&mut citations, &CITE_CLEARSWI);
    }

    // T2*/R2*
    if config.do_t2starmap && config.do_r2starmap {
        sentences.push("T2* and R2* maps were computed from multi-echo magnitude data using the ARLO method (Pei et al., 2015).".to_string());
        add_citation(&mut citations, &CITE_ARLO);
    } else if config.do_t2starmap {
        sentences.push("T2* maps were computed from multi-echo magnitude data using the ARLO method (Pei et al., 2015).".to_string());
        add_citation(&mut citations, &CITE_ARLO);
    } else if config.do_r2starmap {
        sentences.push("R2* maps were computed from multi-echo magnitude data using the ARLO method (Pei et al., 2015).".to_string());
        add_citation(&mut citations, &CITE_ARLO);
    }

    // Build output
    let mut out = String::new();
    out.push_str("# Methods\n\n");
    out.push_str(&sentences.join(" "));
    out.push_str("\n\n");

    // Citations
    if !citations.is_empty() {
        out.push_str("## References\n\n");
        for cite in &citations {
            out.push_str(&format!("- {}\n", cite.text));
        }
    }

    out
}

fn describe_masking(config: &PipelineConfig, sentences: &mut Vec<String>, citations: &mut Vec<&Citation>) {
    if config.mask_sections.is_empty() {
        return;
    }

    // Describe inhomogeneity correction if enabled, with context about what it affects.
    // The RSS-combined magnitude is used for both Magnitude and PhaseQuality masking inputs,
    // while MagnitudeFirst/MagnitudeLast use their specific echo magnitudes.
    if config.inhomogeneity_correction {
        add_citation(citations, &CITE_BIAS);

        let inputs: Vec<MaskingInput> = config.mask_sections.iter().map(|s| s.input).collect();
        let uses_rss = inputs.iter().any(|i| matches!(i, MaskingInput::Magnitude | MaskingInput::PhaseQuality));
        let uses_first = inputs.iter().any(|i| matches!(i, MaskingInput::MagnitudeFirst));
        let uses_last = inputs.iter().any(|i| matches!(i, MaskingInput::MagnitudeLast));

        if uses_rss && !uses_first && !uses_last {
            sentences.push("Inhomogeneity correction (Eckstein et al., 2019) was applied to the RSS-combined magnitude image.".to_string());
        } else if !uses_rss && (uses_first || uses_last) {
            let echo = if uses_first { "first" } else { "last" };
            sentences.push(format!(
                "Inhomogeneity correction (Eckstein et al., 2019) was applied to the {}-echo magnitude image.",
                echo
            ));
        } else {
            sentences.push("Inhomogeneity correction (Eckstein et al., 2019) was applied to the magnitude data.".to_string());
        }
    }

    let section_count = config.mask_sections.len();
    let mut section_descs = Vec::new();

    for section in &config.mask_sections {
        let mut parts = Vec::new();

        // Input description — clarify which magnitude goes into ROMEO
        let input_desc = match section.input {
            MaskingInput::PhaseQuality => {
                add_citation(citations, &CITE_ROMEO);
                if config.inhomogeneity_correction {
                    "the ROMEO phase quality map (computed from phase data and the inhomogeneity-corrected RSS-combined magnitude)"
                } else {
                    "the ROMEO phase quality map (computed from phase data and the RSS-combined magnitude)"
                }
            }
            MaskingInput::Magnitude => {
                if config.inhomogeneity_correction {
                    "the inhomogeneity-corrected RSS-combined magnitude image"
                } else {
                    "the RSS-combined magnitude image"
                }
            }
            MaskingInput::MagnitudeFirst => {
                if config.inhomogeneity_correction {
                    "the inhomogeneity-corrected first-echo magnitude image"
                } else {
                    "the first-echo magnitude image"
                }
            }
            MaskingInput::MagnitudeLast => {
                if config.inhomogeneity_correction {
                    "the inhomogeneity-corrected last-echo magnitude image"
                } else {
                    "the last-echo magnitude image"
                }
            }
        };

        // Generator
        let gen_desc = match &section.generator {
            MaskOp::Threshold { method: MaskThresholdMethod::Otsu, .. } => {
                add_citation(citations, &CITE_OTSU);
                format!("Otsu thresholding (Otsu, 1979) of {}", input_desc)
            }
            MaskOp::Threshold { method: MaskThresholdMethod::Fixed, value } => {
                format!("fixed thresholding (value={:.4}) of {}", value.unwrap_or(0.5), input_desc)
            }
            MaskOp::Threshold { method: MaskThresholdMethod::Percentile, value } => {
                format!("percentile thresholding ({}th percentile) of {}", value.unwrap_or(75.0), input_desc)
            }
            MaskOp::Bet { fractional_intensity } => {
                add_citation(citations, &CITE_BET);
                format!("BET brain extraction (Smith, 2002; f={:.2}) of {}", fractional_intensity, input_desc)
            }
            _ => format!("{} of {}", section.generator, input_desc),
        };
        parts.push(gen_desc);

        // Refinements
        let refinement_descs: Vec<String> = section.refinements.iter().map(|op| match op {
            MaskOp::Erode { iterations } => format!("erosion ({} iteration{})", iterations, if *iterations != 1 { "s" } else { "" }),
            MaskOp::Dilate { iterations } => format!("dilation ({} iteration{})", iterations, if *iterations != 1 { "s" } else { "" }),
            MaskOp::Close { radius } => format!("morphological closing (radius={})", radius),
            MaskOp::FillHoles { max_size: 0 } => "hole-filling".to_string(),
            MaskOp::FillHoles { max_size } => format!("hole-filling (max {} voxels)", max_size),
            MaskOp::GaussianSmooth { sigma_mm } => format!("Gaussian smoothing (sigma={:.1} mm)", sigma_mm),
            _ => format!("{}", op),
        }).collect();

        if !refinement_descs.is_empty() {
            parts.push(format!("followed by {}", join_list(&refinement_descs)));
        }

        section_descs.push(parts.join(", "));
    }

    if section_count == 1 {
        sentences.push(format!("A brain mask was generated using {}.", section_descs[0]));
    } else {
        sentences.push(format!(
            "A brain mask was generated by combining {} mask sections (OR operation): {}.",
            section_count,
            join_list(&section_descs)
        ));
    }
}

fn cite_inline(cite: &Citation) -> &'static str {
    match cite.key {
        "wu2012" => "Wu et al., 2012",
        "schweser2011" => "Schweser et al., 2011",
        "liu2011pdf" => "Liu et al., 2011",
        "wen2014" => "Wen et al., 2014",
        "zhou2014" => "Zhou et al., 2014",
        "kames2018" => "Kames et al., 2018",
        "bilgic2014tv" => "Bilgic et al., 2014",
        "shmueli2009" => "Shmueli et al., 2009",
        "bilgic2014l2" => "Bilgic et al., 2014",
        "liu2011medi" => "Liu et al., 2011",
        "li2015" => "Li et al., 2015",
        _ => cite.key,
    }
}

fn add_citation<'a>(citations: &mut Vec<&'a Citation>, cite: &'a Citation) {
    if !citations.iter().any(|c| c.key == cite.key) {
        citations.push(cite);
    }
}

fn join_list(items: &[String]) -> String {
    match items.len() {
        0 => String::new(),
        1 => items[0].clone(),
        2 => format!("{} and {}", items[0], items[1]),
        _ => {
            let (last, rest) = items.split_last().unwrap();
            format!("{}, and {}", rest.join(", "), last)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── Helper ───

    fn default_cfg() -> PipelineConfig {
        PipelineConfig::default()
    }

    // ─── 17. join_list ───

    #[test]
    fn test_join_list_empty() {
        assert_eq!(join_list(&[]), "");
    }

    #[test]
    fn test_join_list_one() {
        assert_eq!(join_list(&["alpha".into()]), "alpha");
    }

    #[test]
    fn test_join_list_two() {
        assert_eq!(join_list(&["alpha".into(), "beta".into()]), "alpha and beta");
    }

    #[test]
    fn test_join_list_three_plus() {
        let items: Vec<String> = vec!["a".into(), "b".into(), "c".into()];
        assert_eq!(join_list(&items), "a, b, and c");

        let four: Vec<String> = vec!["w".into(), "x".into(), "y".into(), "z".into()];
        assert_eq!(join_list(&four), "w, x, y, and z");
    }

    // ─── 18. cite_inline ───

    #[test]
    fn test_cite_inline_known_keys() {
        assert_eq!(cite_inline(&CITE_VSHARP), "Wu et al., 2012");
        assert_eq!(cite_inline(&CITE_SHARP), "Schweser et al., 2011");
        assert_eq!(cite_inline(&CITE_PDF), "Liu et al., 2011");
        assert_eq!(cite_inline(&CITE_ISMV), "Wen et al., 2014");
        assert_eq!(cite_inline(&CITE_LBV), "Zhou et al., 2014");
        assert_eq!(cite_inline(&CITE_RTS), "Kames et al., 2018");
        assert_eq!(cite_inline(&CITE_TV), "Bilgic et al., 2014");
        assert_eq!(cite_inline(&CITE_TKD), "Shmueli et al., 2009");
        assert_eq!(cite_inline(&CITE_TIKHONOV), "Bilgic et al., 2014");
        assert_eq!(cite_inline(&CITE_MEDI), "Liu et al., 2011");
        assert_eq!(cite_inline(&CITE_ILSQR), "Li et al., 2015");
    }

    #[test]
    fn test_cite_inline_unknown_key_returns_key() {
        let unknown = Citation { key: "unknown2099", text: "Some text" };
        assert_eq!(cite_inline(&unknown), "unknown2099");
    }

    // ─── 1. Default config ───

    #[test]
    fn test_default_config_methods() {
        let cfg = default_cfg();
        let out = generate_methods(&cfg);
        assert!(out.contains("QSM processing was performed using qsmxt.rs"));
        assert!(out.contains("Stewart, 2026"));
        assert!(out.contains("MCPC-3D-S"));
        assert!(out.contains("ROMEO"));
        assert!(out.contains("V-SHARP"));
        assert!(out.contains("RTS"));
        assert!(out.contains("mean-referenced"));
        assert!(out.contains("# Methods"));
        assert!(out.contains("## References"));
    }

    // ─── 2. TGV single-step ───

    #[test]
    fn test_tgv_algorithm() {
        let cfg = PipelineConfig {
            qsm_algorithm: QsmAlgorithm::Tgv,
            ..default_cfg()
        };
        let out = generate_methods(&cfg);
        assert!(out.contains("Total Generalized Variation (TGV)"));
        assert!(out.contains("Langkammer et al., 2015"));
        assert!(out.contains("single step"));
        // Should NOT have separate unwrapping/BF removal sentences
        assert!(!out.contains("Phase unwrapping was performed"));
        assert!(!out.contains("Background field removal was performed"));
    }

    // ─── 3. QSMART ───

    #[test]
    fn test_qsmart_algorithm() {
        let cfg = PipelineConfig {
            qsm_algorithm: QsmAlgorithm::Qsmart,
            ..default_cfg()
        };
        let out = generate_methods(&cfg);
        assert!(out.contains("QSMART"));
        assert!(out.contains("Yaghmaie et al., 2021"));
        assert!(!out.contains("Phase unwrapping was performed"));
        assert!(!out.contains("Background field removal was performed"));
    }

    // ─── 4. Non-TGV with combine_phase ───

    #[test]
    fn test_combine_phase_mcpc3ds() {
        let cfg = PipelineConfig {
            qsm_algorithm: QsmAlgorithm::Rts,
            combine_phase: true,
            ..default_cfg()
        };
        let out = generate_methods(&cfg);
        assert!(out.contains("MCPC-3D-S"));
        assert!(out.contains("Eckstein et al., 2018"));
    }

    #[test]
    fn test_no_combine_phase() {
        let cfg = PipelineConfig {
            combine_phase: false,
            ..default_cfg()
        };
        let out = generate_methods(&cfg);
        assert!(!out.contains("MCPC-3D-S"));
    }

    // ─── 5. Unwrapping algorithms ───

    #[test]
    fn test_unwrapping_romeo() {
        let cfg = PipelineConfig {
            unwrapping_algorithm: Some(UnwrappingAlgorithm::Romeo),
            ..default_cfg()
        };
        let out = generate_methods(&cfg);
        assert!(out.contains("ROMEO"));
        assert!(out.contains("Dymerska et al., 2021"));
    }

    #[test]
    fn test_unwrapping_laplacian() {
        let cfg = PipelineConfig {
            unwrapping_algorithm: Some(UnwrappingAlgorithm::Laplacian),
            ..default_cfg()
        };
        let out = generate_methods(&cfg);
        assert!(out.contains("Laplacian method"));
        assert!(out.contains("Schofield"));
    }

    // ─── 6. BF algorithms ───

    #[test]
    fn test_bf_vsharp() {
        let cfg = PipelineConfig {
            bf_algorithm: Some(BfAlgorithm::Vsharp),
            ..default_cfg()
        };
        let out = generate_methods(&cfg);
        assert!(out.contains("V-SHARP"));
        assert!(out.contains("Wu et al., 2012"));
    }

    #[test]
    fn test_bf_pdf() {
        let cfg = PipelineConfig {
            bf_algorithm: Some(BfAlgorithm::Pdf),
            ..default_cfg()
        };
        let out = generate_methods(&cfg);
        assert!(out.contains("PDF"));
        assert!(out.contains("Liu et al., 2011"));
    }

    #[test]
    fn test_bf_lbv() {
        let cfg = PipelineConfig {
            bf_algorithm: Some(BfAlgorithm::Lbv),
            ..default_cfg()
        };
        let out = generate_methods(&cfg);
        assert!(out.contains("LBV"));
        assert!(out.contains("Zhou et al., 2014"));
    }

    #[test]
    fn test_bf_ismv() {
        let cfg = PipelineConfig {
            bf_algorithm: Some(BfAlgorithm::Ismv),
            ..default_cfg()
        };
        let out = generate_methods(&cfg);
        assert!(out.contains("iSMV"));
        assert!(out.contains("Wen et al., 2014"));
    }

    #[test]
    fn test_bf_sharp() {
        let cfg = PipelineConfig {
            bf_algorithm: Some(BfAlgorithm::Sharp),
            ..default_cfg()
        };
        let out = generate_methods(&cfg);
        assert!(out.contains("SHARP"));
        assert!(out.contains("Schweser et al., 2011"));
    }

    // ─── 7. QSM algorithms (dipole inversion) ───

    fn test_qsm_alg(alg: QsmAlgorithm, expected_name: &str, expected_cite: &str) {
        let cfg = PipelineConfig {
            qsm_algorithm: alg,
            ..default_cfg()
        };
        let out = generate_methods(&cfg);
        assert!(out.contains("Dipole inversion was performed using"), "Missing dipole inversion sentence for {:?}", alg);
        assert!(out.contains(expected_name), "Missing algorithm name '{}' for {:?}", expected_name, alg);
        assert!(out.contains(expected_cite), "Missing citation '{}' for {:?}", expected_cite, alg);
    }

    #[test]
    fn test_qsm_rts() {
        test_qsm_alg(QsmAlgorithm::Rts, "RTS", "Kames et al., 2018");
    }

    #[test]
    fn test_qsm_tv() {
        test_qsm_alg(QsmAlgorithm::Tv, "Total Variation", "Bilgic et al., 2014");
    }

    #[test]
    fn test_qsm_tkd() {
        test_qsm_alg(QsmAlgorithm::Tkd, "TKD", "Shmueli et al., 2009");
    }

    #[test]
    fn test_qsm_tsvd() {
        test_qsm_alg(QsmAlgorithm::Tsvd, "TSVD", "Shmueli et al., 2009");
    }

    #[test]
    fn test_qsm_tikhonov() {
        test_qsm_alg(QsmAlgorithm::Tikhonov, "Tikhonov", "Bilgic et al., 2014");
    }

    #[test]
    fn test_qsm_nltv() {
        test_qsm_alg(QsmAlgorithm::Nltv, "NLTV", "Kames et al., 2018");
    }

    #[test]
    fn test_qsm_medi() {
        test_qsm_alg(QsmAlgorithm::Medi, "MEDI", "Liu et al., 2011");
    }

    #[test]
    fn test_qsm_ilsqr() {
        test_qsm_alg(QsmAlgorithm::Ilsqr, "iLSQR", "Li et al., 2015");
    }

    // ─── 8. do_qsm = false ───

    #[test]
    fn test_do_qsm_false() {
        let cfg = PipelineConfig {
            do_qsm: false,
            ..default_cfg()
        };
        let out = generate_methods(&cfg);
        assert!(out.contains("MRI processing was performed using qsmxt.rs"));
        assert!(!out.contains("QSM processing"));
        assert!(!out.contains("Dipole inversion"));
        assert!(!out.contains("Phase unwrapping was performed"));
        assert!(!out.contains("Background field removal"));
        assert!(!out.contains("susceptibility map was mean-referenced"));
    }

    // ─── 9. do_swi ───

    #[test]
    fn test_do_swi() {
        let cfg = PipelineConfig {
            do_swi: true,
            ..default_cfg()
        };
        let out = generate_methods(&cfg);
        assert!(out.contains("CLEAR-SWI"));
        assert!(out.contains("Eckstein et al., 2024"));
    }

    #[test]
    fn test_no_swi() {
        let cfg = default_cfg();
        let out = generate_methods(&cfg);
        assert!(!out.contains("CLEAR-SWI"));
    }

    // ─── 10. T2* and R2* both ───

    #[test]
    fn test_t2star_and_r2star_both() {
        let cfg = PipelineConfig {
            do_t2starmap: true,
            do_r2starmap: true,
            ..default_cfg()
        };
        let out = generate_methods(&cfg);
        assert!(out.contains("T2* and R2* maps"));
        assert!(out.contains("ARLO"));
        assert!(out.contains("Pei et al., 2015"));
    }

    // ─── 11. T2* only, R2* only ───

    #[test]
    fn test_t2star_only() {
        let cfg = PipelineConfig {
            do_t2starmap: true,
            do_r2starmap: false,
            ..default_cfg()
        };
        let out = generate_methods(&cfg);
        assert!(out.contains("T2* maps were computed"));
        assert!(!out.contains("R2* maps"));
        assert!(!out.contains("T2* and R2*"));
    }

    #[test]
    fn test_r2star_only() {
        let cfg = PipelineConfig {
            do_t2starmap: false,
            do_r2starmap: true,
            ..default_cfg()
        };
        let out = generate_methods(&cfg);
        assert!(out.contains("R2* maps were computed"));
        assert!(!out.contains("T2* maps"));
        assert!(!out.contains("T2* and R2*"));
    }

    // ─── 12. QsmReference ───

    #[test]
    fn test_qsm_reference_mean() {
        let cfg = PipelineConfig {
            qsm_reference: QsmReference::Mean,
            ..default_cfg()
        };
        let out = generate_methods(&cfg);
        assert!(out.contains("mean-referenced within the brain mask"));
    }

    #[test]
    fn test_qsm_reference_none() {
        let cfg = PipelineConfig {
            qsm_reference: QsmReference::None,
            ..default_cfg()
        };
        let out = generate_methods(&cfg);
        assert!(out.contains("No susceptibility referencing was applied"));
    }

    // ─── 13. Masking generators ───

    #[test]
    fn test_masking_otsu_threshold() {
        let cfg = PipelineConfig {
            mask_sections: vec![MaskSection {
                input: MaskingInput::Magnitude,
                generator: MaskOp::Threshold { method: MaskThresholdMethod::Otsu, value: None },
                refinements: vec![],
            }],
            ..default_cfg()
        };
        let out = generate_methods(&cfg);
        assert!(out.contains("Otsu thresholding"));
        assert!(out.contains("Otsu, 1979"));
    }

    #[test]
    fn test_masking_bet() {
        let cfg = PipelineConfig {
            mask_sections: vec![MaskSection {
                input: MaskingInput::Magnitude,
                generator: MaskOp::Bet { fractional_intensity: 0.35 },
                refinements: vec![],
            }],
            ..default_cfg()
        };
        let out = generate_methods(&cfg);
        assert!(out.contains("BET brain extraction"));
        assert!(out.contains("Smith, 2002"));
        assert!(out.contains("f=0.35"));
    }

    #[test]
    fn test_masking_fixed_threshold() {
        let cfg = PipelineConfig {
            mask_sections: vec![MaskSection {
                input: MaskingInput::Magnitude,
                generator: MaskOp::Threshold { method: MaskThresholdMethod::Fixed, value: Some(0.3) },
                refinements: vec![],
            }],
            ..default_cfg()
        };
        let out = generate_methods(&cfg);
        assert!(out.contains("fixed thresholding"));
        assert!(out.contains("value=0.3"));
    }

    #[test]
    fn test_masking_percentile_threshold() {
        let cfg = PipelineConfig {
            mask_sections: vec![MaskSection {
                input: MaskingInput::Magnitude,
                generator: MaskOp::Threshold { method: MaskThresholdMethod::Percentile, value: Some(80.0) },
                refinements: vec![],
            }],
            ..default_cfg()
        };
        let out = generate_methods(&cfg);
        assert!(out.contains("percentile thresholding"));
        assert!(out.contains("80th percentile"));
    }

    // ─── 14. Inhomogeneity correction + different masking inputs ───

    #[test]
    fn test_inhomogeneity_correction_magnitude() {
        let cfg = PipelineConfig {
            inhomogeneity_correction: true,
            mask_sections: vec![MaskSection {
                input: MaskingInput::Magnitude,
                generator: MaskOp::Threshold { method: MaskThresholdMethod::Otsu, value: None },
                refinements: vec![],
            }],
            ..default_cfg()
        };
        let out = generate_methods(&cfg);
        assert!(out.contains("Inhomogeneity correction"));
        assert!(out.contains("Eckstein et al., 2019"));
        assert!(out.contains("RSS-combined magnitude image"));
    }

    #[test]
    fn test_inhomogeneity_correction_phase_quality() {
        let cfg = PipelineConfig {
            inhomogeneity_correction: true,
            mask_sections: vec![MaskSection {
                input: MaskingInput::PhaseQuality,
                generator: MaskOp::Threshold { method: MaskThresholdMethod::Otsu, value: None },
                refinements: vec![],
            }],
            ..default_cfg()
        };
        let out = generate_methods(&cfg);
        assert!(out.contains("Inhomogeneity correction"));
        assert!(out.contains("RSS-combined magnitude image"));
        assert!(out.contains("ROMEO phase quality map"));
        assert!(out.contains("inhomogeneity-corrected RSS-combined magnitude"));
    }

    #[test]
    fn test_inhomogeneity_correction_magnitude_first() {
        let cfg = PipelineConfig {
            inhomogeneity_correction: true,
            mask_sections: vec![MaskSection {
                input: MaskingInput::MagnitudeFirst,
                generator: MaskOp::Threshold { method: MaskThresholdMethod::Otsu, value: None },
                refinements: vec![],
            }],
            ..default_cfg()
        };
        let out = generate_methods(&cfg);
        assert!(out.contains("first-echo magnitude image"));
    }

    #[test]
    fn test_inhomogeneity_correction_magnitude_last() {
        let cfg = PipelineConfig {
            inhomogeneity_correction: true,
            mask_sections: vec![MaskSection {
                input: MaskingInput::MagnitudeLast,
                generator: MaskOp::Threshold { method: MaskThresholdMethod::Otsu, value: None },
                refinements: vec![],
            }],
            ..default_cfg()
        };
        let out = generate_methods(&cfg);
        assert!(out.contains("last-echo magnitude image"));
    }

    #[test]
    fn test_no_inhomogeneity_correction() {
        let cfg = PipelineConfig {
            inhomogeneity_correction: false,
            mask_sections: vec![MaskSection {
                input: MaskingInput::Magnitude,
                generator: MaskOp::Threshold { method: MaskThresholdMethod::Otsu, value: None },
                refinements: vec![],
            }],
            ..default_cfg()
        };
        let out = generate_methods(&cfg);
        assert!(!out.contains("Inhomogeneity correction"));
        assert!(out.contains("the RSS-combined magnitude image"));
        assert!(!out.contains("inhomogeneity-corrected"));
    }

    #[test]
    fn test_inhomogeneity_correction_mixed_rss_and_first() {
        // When both RSS-type and first/last inputs are used, should say "magnitude data"
        let cfg = PipelineConfig {
            inhomogeneity_correction: true,
            mask_sections: vec![
                MaskSection {
                    input: MaskingInput::Magnitude,
                    generator: MaskOp::Threshold { method: MaskThresholdMethod::Otsu, value: None },
                    refinements: vec![],
                },
                MaskSection {
                    input: MaskingInput::MagnitudeFirst,
                    generator: MaskOp::Threshold { method: MaskThresholdMethod::Otsu, value: None },
                    refinements: vec![],
                },
            ],
            ..default_cfg()
        };
        let out = generate_methods(&cfg);
        assert!(out.contains("applied to the magnitude data"));
    }

    // ─── 15. Multiple mask sections (OR combination) ───

    #[test]
    fn test_multiple_mask_sections_or() {
        let cfg = PipelineConfig {
            mask_sections: vec![
                MaskSection {
                    input: MaskingInput::Magnitude,
                    generator: MaskOp::Threshold { method: MaskThresholdMethod::Otsu, value: None },
                    refinements: vec![],
                },
                MaskSection {
                    input: MaskingInput::PhaseQuality,
                    generator: MaskOp::Threshold { method: MaskThresholdMethod::Otsu, value: None },
                    refinements: vec![],
                },
            ],
            ..default_cfg()
        };
        let out = generate_methods(&cfg);
        assert!(out.contains("combining"));
        assert!(out.contains("2 mask sections"));
        assert!(out.contains("OR operation"));
    }

    // ─── 16. Refinements ───

    #[test]
    fn test_refinement_erode() {
        let cfg = PipelineConfig {
            mask_sections: vec![MaskSection {
                input: MaskingInput::Magnitude,
                generator: MaskOp::Threshold { method: MaskThresholdMethod::Otsu, value: None },
                refinements: vec![MaskOp::Erode { iterations: 3 }],
            }],
            ..default_cfg()
        };
        let out = generate_methods(&cfg);
        assert!(out.contains("erosion (3 iterations)"));
    }

    #[test]
    fn test_refinement_erode_singular() {
        let cfg = PipelineConfig {
            mask_sections: vec![MaskSection {
                input: MaskingInput::Magnitude,
                generator: MaskOp::Threshold { method: MaskThresholdMethod::Otsu, value: None },
                refinements: vec![MaskOp::Erode { iterations: 1 }],
            }],
            ..default_cfg()
        };
        let out = generate_methods(&cfg);
        assert!(out.contains("erosion (1 iteration)"));
        assert!(!out.contains("iterations)"));
    }

    #[test]
    fn test_refinement_dilate() {
        let cfg = PipelineConfig {
            mask_sections: vec![MaskSection {
                input: MaskingInput::Magnitude,
                generator: MaskOp::Threshold { method: MaskThresholdMethod::Otsu, value: None },
                refinements: vec![MaskOp::Dilate { iterations: 2 }],
            }],
            ..default_cfg()
        };
        let out = generate_methods(&cfg);
        assert!(out.contains("dilation (2 iterations)"));
    }

    #[test]
    fn test_refinement_close() {
        let cfg = PipelineConfig {
            mask_sections: vec![MaskSection {
                input: MaskingInput::Magnitude,
                generator: MaskOp::Threshold { method: MaskThresholdMethod::Otsu, value: None },
                refinements: vec![MaskOp::Close { radius: 5 }],
            }],
            ..default_cfg()
        };
        let out = generate_methods(&cfg);
        assert!(out.contains("morphological closing (radius=5)"));
    }

    #[test]
    fn test_refinement_fill_holes_zero() {
        let cfg = PipelineConfig {
            mask_sections: vec![MaskSection {
                input: MaskingInput::Magnitude,
                generator: MaskOp::Threshold { method: MaskThresholdMethod::Otsu, value: None },
                refinements: vec![MaskOp::FillHoles { max_size: 0 }],
            }],
            ..default_cfg()
        };
        let out = generate_methods(&cfg);
        assert!(out.contains("hole-filling"));
        assert!(!out.contains("voxels"));
    }

    #[test]
    fn test_refinement_fill_holes_nonzero() {
        let cfg = PipelineConfig {
            mask_sections: vec![MaskSection {
                input: MaskingInput::Magnitude,
                generator: MaskOp::Threshold { method: MaskThresholdMethod::Otsu, value: None },
                refinements: vec![MaskOp::FillHoles { max_size: 500 }],
            }],
            ..default_cfg()
        };
        let out = generate_methods(&cfg);
        assert!(out.contains("hole-filling (max 500 voxels)"));
    }

    #[test]
    fn test_refinement_gaussian() {
        let cfg = PipelineConfig {
            mask_sections: vec![MaskSection {
                input: MaskingInput::Magnitude,
                generator: MaskOp::Threshold { method: MaskThresholdMethod::Otsu, value: None },
                refinements: vec![MaskOp::GaussianSmooth { sigma_mm: 2.5 }],
            }],
            ..default_cfg()
        };
        let out = generate_methods(&cfg);
        assert!(out.contains("Gaussian smoothing (sigma=2.5 mm)"));
    }

    #[test]
    fn test_multiple_refinements_joined() {
        let cfg = PipelineConfig {
            mask_sections: vec![MaskSection {
                input: MaskingInput::Magnitude,
                generator: MaskOp::Threshold { method: MaskThresholdMethod::Otsu, value: None },
                refinements: vec![
                    MaskOp::Dilate { iterations: 1 },
                    MaskOp::FillHoles { max_size: 0 },
                    MaskOp::Erode { iterations: 1 },
                ],
            }],
            ..default_cfg()
        };
        let out = generate_methods(&cfg);
        assert!(out.contains("followed by"));
        // With 3 items, join_list uses ", and" format
        assert!(out.contains(", and "));
    }

    // ─── Empty mask sections ───

    #[test]
    fn test_empty_mask_sections() {
        let cfg = PipelineConfig {
            mask_sections: vec![],
            ..default_cfg()
        };
        let out = generate_methods(&cfg);
        assert!(!out.contains("A brain mask was generated"));
    }

    // ─── Comprehensive: all features on ───

    #[test]
    fn test_all_features_enabled() {
        let cfg = PipelineConfig {
            do_qsm: true,
            do_swi: true,
            do_t2starmap: true,
            do_r2starmap: true,
            ..default_cfg()
        };
        let out = generate_methods(&cfg);
        assert!(out.contains("QSM processing"));
        assert!(out.contains("CLEAR-SWI"));
        assert!(out.contains("T2* and R2* maps"));
        assert!(out.contains("## References"));
    }
}
