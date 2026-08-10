#!/usr/bin/env bash
#
# check-theme-colors.sh — no raw colour outside the theme module.
#
# WHAT THIS GATE IS FOR
# =====================
#
# pdfce's appearance is data, in `crates/pdfce-gui/src/theme.rs`, so that
# changing the look is a one-file edit instead of a sweep through 27,000
# lines. That property survives only if new colours keep landing there.
#
# The failure this catches is not a crash and not a wrong colour — it is
# a colour that works fine today and is invisible to the next restyle.
# Six months of "just this one literal, it's only a hairline" and the
# theme module is decoration: the app has a palette AND a scattering of
# hard-coded colours that no longer match it, and nobody finds out until
# someone switches preset and half the canvas stays light.
#
# Same shape as `check-ui-strings.sh`, deliberately. That gate keeps every
# operator-visible string in `ui_text.rs`, which is what makes rewording
# safe; this one keeps every colour in `theme.rs`, which is what makes
# restyling safe. Both are cheap greps that catch a class of drift no
# compiler and no unit test can see.
#
# ★ THE ESCAPE HATCH IS NOT A LOOPHOLE — IT IS THE POINT
# ======================================================
#
# Two colours in this application are written INTO THE PDF:
#
#   * `markup_color` — the colour of an annotation the operator authors
#   * `prop_color`   — the same, from the properties panel
#
# Those are the operator's choice about document content. They are not
# chrome, and a theme must never move them: restyling the application
# would change the colour of markup about to be committed to a file, and
# the change would only become visible after saving.
#
# So a line carrying the comment `DOCUMENT COLOUR:` is allowed. The gate's
# job is to catch the colour someone forgot to name — not to forbid the
# two that must stay exactly where they are. A gate with no way to say
# "this one is different" gets switched off the first time it is right
# about the wrong thing.
#
# EXIT CODES
# ==========
#   0  clean
#   1  at least one un-named colour outside the theme module
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1

GUI_SRC="crates/pdfce-gui/src"
THEME="$GUI_SRC/theme.rs"

if [ ! -f "$THEME" ]; then
  echo "theme-colors: FAIL — $THEME does not exist; the gate has nothing to protect"
  exit 1
fi

# `from_gray` and the named constants (`Color32::RED`) count too: a
# literal grey is exactly as invisible to a restyle as a literal blue.
PATTERN='Color32::(from_rgb|from_rgba_unmultiplied|from_rgba_premultiplied|from_gray|BLACK|WHITE|GRAY|LIGHT_GRAY|DARK_GRAY|RED|GREEN|BLUE|YELLOW|BROWN|GOLD|KHAKI|ORANGE|PURPLE)'
#
# `TRANSPARENT` is deliberately NOT in that list. It is the absence of a
# colour, not a choice of one — no theme would ever change it, and
# requiring a marker on every `Shape::convex_polygon(.., TRANSPARENT, ..)`
# would train people to add markers without reading them, which is how a
# gate stops being read at all.

# The marker is honoured on the offending line OR on any of the seven
# lines above it, because that is where a comment explaining a line
# actually goes — and an explanation worth writing is usually longer than
# one line. The first version allowed three, which rejected a five-line
# comment saying exactly what the gate asks for. A gate that only accepts a trailing marker forces the
# explanation onto the end of the line it is explaining, where it does
# not fit and gets deleted by the next reformat.
offenders=$(
  find "$GUI_SRC" -name '*.rs' ! -path "$THEME" -print0 \
  | xargs -0 awk -v pat="$PATTERN" '
      FNR == 1 { for (i = 0; i < 8; i++) recent[i] = "" }
      {
        marked = 0
        for (i = 0; i < 8; i++)
          if (recent[i] ~ /DOCUMENT COLOUR:|NOT A THEME COLOUR:/) marked = 1
        if ($0 ~ /DOCUMENT COLOUR:|NOT A THEME COLOUR:/) marked = 1
        # A comment mentioning a colour is prose, not a drawn colour.
        is_comment = ($0 ~ /^[ \t]*(\/\/|\*)/)
        if (!marked && !is_comment && $0 ~ pat)
          printf "%s:%d:%s\n", FILENAME, FNR, $0
        for (i = 7; i > 0; i--) recent[i] = recent[i-1]
        recent[0] = $0
      }
    '
)
if [ -n "$offenders" ]; then
  echo "theme-colors: FAIL — raw colours outside $THEME:"
  echo "$offenders" | sed 's/^/  /'
  cat <<'EOF'

Every colour pdfce draws belongs to a named role in theme.rs, so that a
restyle is one file rather than a sweep. Add a role to `Palette` and use
it here.

If this colour is written INTO THE DOCUMENT rather than drawn as chrome —
annotation colour, appearance-stream colour, anything that reaches a
saved file — it must NOT be themed. Mark the line:

    // DOCUMENT COLOUR: <why it reaches the file>

If it is neither chrome nor document content (arithmetic on an existing
colour, a test fixture asserting an exact value), mark it:

    // NOT A THEME COLOUR: <why>
EOF
  exit 1
fi

echo "theme-colors: clean — every colour is a named role in theme.rs"
exit 0
