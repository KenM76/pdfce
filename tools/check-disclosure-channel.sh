#!/usr/bin/env bash
# check-disclosure-channel.sh — enforce ARCHITECTURE.md §4.1(P):
# a field carrying a DISCLOSURE OBLIGATION is assigned only inside its own
# traced setter, so every note that reaches the operator also reaches the
# observation harness.
#
# WHY THIS GATE EXISTS
# --------------------
# `PdfceApp::edit_note` is the status bar's narrator. Rule 4's obligations
# land in it: the inferred value, the cleared selection, the released group
# name, the refusal a verb reported instead of succeeding.
#
# The `pending_note` drain has traced its share since Pass 34.2, with a doc
# comment stating exactly why: "a disclosure that silently stopped firing
# would look identical to one that fired and said something." That reasoning
# was right and it covered A THIRD OF THE CHANNEL. Every canvas-level
# disclosure — delete, node edit, subpath edit, markup, copy, refusals —
# assigned `self.edit_note` directly at 34 sites and emitted nothing a
# behavioural harness could see.
#
# It was found on 2026-08-09 by driving a newly-added disclosure, seeing no
# `edit-note` line, and briefly concluding the disclosure had not fired. It
# had. The instrument was the silent thing — the FOURTH recorded instance in
# this project of a component built for observability being itself
# unobservable, and the first that was recursive.
#
# The fix (one traced `set_edit_note`) is one refactor away from decaying:
# nothing stops the next feature writing `self.edit_note = Some(...)` again,
# and the failure would be invisible in exactly the way the original was.
# Hence a gate rather than a comment.
#
# WHAT IT CHECKS
# --------------
# Exactly one bare assignment to each guarded field may exist — the one
# INSIDE that field's own setter. Anything else is a producer bypassing the
# choke point.
#
# `= None` is NOT counted. Clearing a note is not a disclosure; it happens
# on document open and on state resets, and tracing "the narrator was
# emptied" would be noise the harness has to filter rather than signal.
#
# EXIT CODES
# ----------
# 0  clean — every disclosure goes through its setter.
# 1  one or more producers bypass the choke point; each is printed with its
#    file:line so it can be fixed rather than hunted.
#
# USAGE
# -----
#     bash tools/check-disclosure-channel.sh

set -uo pipefail
cd "$(dirname "$0")/.."

# field name : the setter that owns it : how many bare assignments are legal
# (always 1 — the one inside the setter itself).
GUARDED=(
  "edit_note:set_edit_note"
  # Added with the field, Pass 52.2, rather than retrofitted after a
  # disclosure was found to be invisible — which is how the first entry got
  # here. `dxf_export_result` carries rule-4 obligations of its own: text
  # pdfce could not read and dropped, images it has no representation for,
  # and a paper-scale export the operator agreed to and will not remember
  # agreeing to. Same failure mode as `edit_note`, so the same choke point.
  "dxf_export_result:set_dxf_export_result"
)

fail=0
for entry in "${GUARDED[@]}"; do
  field="${entry%%:*}"
  setter="${entry##*:}"

  # `self.<field> = Some(` — the assignment form that carries a disclosure.
  # Deliberately NOT matching `= None`: see the header.
  hits=$(grep -rn "self\.${field} = Some(" crates/pdfce-gui/src/ 2>/dev/null || true)
  count=$(printf '%s' "$hits" | grep -c . || true)

  if [ "$count" -gt 1 ]; then
    echo "disclosure-channel: ${field} is assigned at ${count} sites; only the one"
    echo "  inside \`${setter}\` may exist. Every other producer bypasses the trace,"
    echo "  so its note reaches the operator and NOT the observation harness."
    echo "  Route each through \`self.${setter}(...)\`:"
    printf '%s\n' "$hits" | sed 's/^/    /'
    fail=1
  elif [ "$count" -eq 0 ]; then
    # Zero is not success — it means the setter itself stopped assigning, or
    # the field was renamed and this gate is now watching nothing. A gate
    # that silently guards a field that no longer exists is the false-green
    # shape R106 has been amended four times over.
    echo "disclosure-channel: found NO assignment to ${field}."
    echo "  Either \`${setter}\` no longer assigns it, or the field was renamed and"
    echo "  this gate is watching a name that does not exist. Both make the gate"
    echo "  green while guarding nothing — fix the gate, do not delete this line."
    fail=1
  fi
done

if [ "$fail" -eq 0 ]; then
  echo "disclosure-channel: clean — every operator-facing note goes through its traced setter"
fi
exit "$fail"
