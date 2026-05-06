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
