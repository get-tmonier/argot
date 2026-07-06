# argot README demo — re-render with `docs/demo/render.sh` (needs VHS).
#
# The tape assumes it runs inside a repo where `argot fit` has already been run
# and an out-of-voice hunk (docs/demo/receipts.py) is staged. render.sh sets
# that up against the pinned FastAPI benchmark checkout, so the GIF shows the
# exact hit the README quotes.

Output demo.gif

Require argot

Set Shell "bash"
Set FontSize 20
Set Width 1280
# Sized to hug the output instead of a tall half-empty terminal.
Set Height 470
Set Padding 20
Set Margin 10
Set BorderRadius 10
Set Theme "Catppuccin Mocha"
Set TypingSpeed 45ms

Sleep 500ms
Type "argot check --staged"
Sleep 500ms
Enter
Sleep 4500ms
