# MEMORY.md — pdfce-engineer index

- [KenAgent decision protocol](feedback_kenagent_decisions.md) — route technical decisions through autonomous-builder; archive Markdown to docs/decisions/
- [Inkscape parity scope](project_inkscape_parity.md) — 2026-07-30: scope = Acrobat Pro + Inkscape vector-editing parity; Inkscape is GPL, reference-only
- [Launch on completion](feedback_launch_on_completion.md) — always launch the app (GUI window / CLI demo) when a work unit completes, not just report
- [Fuzz ASan DLL](reference_fuzz_asan_dll.md) — Windows cargo-fuzz needs MSVC clang_rt.asan DLL on PATH or dies with STATUS_DLL_NOT_FOUND
- [clap Windows stack](reference_clap_windows_stack.md) — debug pdfce-cli overflows main-thread stack as clap tree grows; fix = run on 16MB worker thread
- [Pass 7.1 forms](project_pass71_forms.md) — flatten/FDF/XFDF/choice/regenerate/JS-histogram design decisions (append-not-rewrite flatten, R48 via deletion)
- [Pass 8 redaction](project_pass8_redaction.md) — surgery interpreter, container decomposition, forced full-rewrite, ui-spec P0 (mark-vs-apply, refusal-ack gate, scrub-rides-rewrite)
- [Pass 11 render-parity](project_pass11_render_parity.md) — full-page pdfium pixel-parity gate (tools/render-parity); band=0.0294; buckets 2840/49/1; residuals: A019 f32-coord bug + DeviceCMYK colorimetry 3.0x
- [Decision 012 supplied fonts](project_decision012_supplied_fonts.md) — first cut shipped (render/CLI/GUI); R61–R65 collide with existing R61 (renumber before filing); GlyphSource migration; skrifa face-name API
- [Decision 013 Pass B xref recovery](project_decision013_pass_b_xref_recovery.md) — rebuild-by-scan shipped; 566/1109 real-world converted, 0 regressions; standing-rule ~R62 flag; gen-65536 recover-anyway deviation flagged
- [Engineer does the observing](feedback_engineer_does_the_observing.md) — Ken won't beta-test; verify operator-facing behavior in the running app yourself, never stall waiting on him
- [Pass 17 live-edit rendering](project_pass17_live_edit_rendering.md) — GUI rendered base revision not edit overlay, hiding 14 Passes of editing work; decisions 017 (egui_tiles adopted) + 018 (fix)
