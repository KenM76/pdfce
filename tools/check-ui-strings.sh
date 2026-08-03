#!/usr/bin/env bash
# check-ui-strings.sh — enforce decision 002 rule R1:
# every OPERATOR-VISIBLE string in pdfce-gui lives in ui_text.rs.
#
# WHY THIS IS A SCRIPT AND NOT AN INLINE CI GREP
# ----------------------------------------------
# It used to be an inline grep in .github/workflows/ci.yml. That grep was
# FAILING AT BASELINE on 140 hits and had been for some time, which means the
# rule it exists to enforce was not being enforced at all. Worse than useless:
# when it was finally read, the noise was hiding exactly one genuine violation
# (the Measure sub-tool names "Linear" / "Radius/Diameter" / "Set Scale", which
# are drawn on the toolbar and lived as bare literals in main.rs).
#
# A gate that cannot pass guards nothing, and it trains everyone who sees it
# red to ignore it. So the rule is now expressed somewhere it can be RUN
# LOCALLY before pushing, tuned until it is honestly green, and read by a
# human when it fires.
#
# WHAT IT SKIPS, AND WHY EACH EXCLUSION IS PRINCIPLED
# ---------------------------------------------------
# 1. `ui_text.rs` itself — that IS the catalog.
#
# 2. Everything from `#[cfg(test)]` to end of file. Test assertion messages
#    are prose, but they are never rendered to an operator; they are read by
#    whoever is staring at a failing test. Including them was the single
#    biggest source of the 140-hit noise floor (125 of them). This codebase
#    puts its test module last, so a truncation is exact rather than a guess.
#
#    LIMIT, found the embarrassing way: because this truncates, any non-test
#    code placed AFTER the test module is invisible to the checker. It surfaced
#    while planting a deliberate violation to prove the gate still bites — the
#    plant was appended to end-of-file, the gate stayed green, and for a moment
#    it looked as though the fix had produced a gate that could only pass.
#    Re-planting it above `#[cfg(test)]` caught it correctly.
#
#    Two lessons kept rather than quietly dropped: verify a gate by making it
#    FAIL on purpose, never only by making it pass; and Rust after `mod tests`
#    is unusual but legal, so if that convention is ever broken this exclusion
#    silently stops covering the tail of the file.
#
# 3. Lines inside an `impl ... Display for ...` block. `Display` formats
#    DIAGNOSTIC text — an error's own description of itself — which is a
#    different audience and a different lifecycle from UI copy. Tracked by
#    brace depth so it ends where the impl ends, not at a blank line.
#
#    CAVEAT, stated rather than hidden: if an error's `Display` output is ever
#    shown verbatim in the GUI, that string HAS become operator-visible and
#    belongs in the catalog. This exclusion is not permission to route UI text
#    through an error type.
#
# 4. Comment-only lines, and any line carrying `// ui-text-exempt: <reason>`.
#
# HEURISTIC AND ITS KNOWN LIMIT
# -----------------------------
# It flags string literals containing whitespace. That is a proxy for "prose",
# and it is leaky in both directions: "Linear" is operator-visible but has no
# space, so the gate would never have caught it (it was moved to the catalog
# anyway, because the RULE is about visibility, not about what grep can see);
# and a whitespace-bearing literal may be an egui id or a format spec. The
# exemption comment exists for the latter. Do not mistake a green run here for
# proof that the catalog is complete.
#
# Usage:  tools/check-ui-strings.sh
# Exit:   0 clean, 1 violations found (printed).

set -euo pipefail

SRC_DIR="crates/pdfce-gui/src"
if [ ! -d "$SRC_DIR" ]; then
    echo "error: run from the repository root (no $SRC_DIR)" >&2
    exit 2
fi

hits=""

for file in "$SRC_DIR"/*.rs; do
    case "$(basename "$file")" in
        ui_text.rs) continue ;;   # exclusion 1: this is the catalog
    esac

    file_hits=$(awk -v fname="$file" '
        # exclusion 2: stop at the test module — everything after is test-only.
        /^#\[cfg\(test\)\]/ { exit }

        {
            line = $0

            # exclusion 3: track an `impl ... Display for ...` block by brace
            # depth, so the skip ends exactly where the impl does.
            if (in_display) {
                depth += gsub(/\{/, "{", line)
                depth -= gsub(/\}/, "}", line)
                if (depth <= 0) { in_display = 0 }
                next
            }
            if (line ~ /impl[[:space:]].*Display[[:space:]]+for[[:space:]]/) {
                in_display = 1
                depth = gsub(/\{/, "{", line) - gsub(/\}/, "}", line)
                if (depth <= 0) { in_display = 0 }
                next
            }

            # exclusion 4: comment-only lines, and explicit exemptions.
            #
            # An exemption counts either on the offending line itself, or
            # anywhere in the contiguous comment block immediately above it.
            # The block form exists because this project asks for reasons, not
            # tokens: "// ui-text-exempt: stderr diagnostic" trailing a line is
            # fine, but a real justification runs several lines and belongs
            # above the code rather than smeared past column 100.
            if (line ~ /^[[:space:]]*\/\//) {
                if (line ~ /ui-text-exempt:/) { block_exempt = 1 }
                next            # still a comment line: block continues
            }
            if (line ~ /ui-text-exempt:/) { block_exempt = 0; next }
            if (block_exempt) { block_exempt = 0; next }

            # The heuristic: a string literal containing whitespace.
            #
            # Scan the line character by character rather than regex-matching
            # `"[^"]*[[:space:]][^"]*"`. That pattern is wrong in a way that
            # matters: it happily starts at one literal CLOSING quote and ends
            # at the next literal OPENING quote, so `"svg" | "?xml"` reads as a
            # single literal containing " | ". Three of the four remaining hits
            # when this checker was first run were exactly that artefact —
            # i.e. most of what was left after the real exclusions was the
            # detector misreading Rust, not the code violating the rule.
            #
            # A scanner that toggles on unescaped quotes cannot make that
            # mistake, because it knows which quotes open and which close.
            n = length(line)
            in_str = 0
            lit = ""
            for (i = 1; i <= n; i++) {
                ch = substr(line, i, 1)
                if (in_str) {
                    if (ch == "\\") { i++; lit = lit "x"; continue }
                    if (ch == "\"") {
                        in_str = 0
                        if (lit ~ /[[:space:]]/) {
                            printf "%s:%d:%s\n", fname, NR, line
                            next
                        }
                        continue
                    }
                    lit = lit ch
                } else if (ch == "\"") {
                    in_str = 1
                    lit = ""
                }
            }
        }
    ' "$file")

    if [ -n "$file_hits" ]; then
        hits="${hits}${file_hits}
"
    fi
done

hits=$(printf '%s' "$hits" | sed '/^$/d')

if [ -n "$hits" ]; then
    printf '%s\n' "$hits"
    count=$(printf '%s\n' "$hits" | grep -c '^')
    echo ""
    echo "error: $count user-facing string literal(s) outside ui_text.rs."
    echo "Move each into the catalog (decision 002 R1), or, if it is genuinely"
    echo "not operator-visible, append '// ui-text-exempt: <reason>' to the line."
    exit 1
fi

echo "ui-strings: clean — no operator-visible literals outside ui_text.rs"
