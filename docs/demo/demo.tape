# argot README demo — re-render with `docs/demo/render.sh` (needs VHS).
#
# The tape assumes it runs inside a repo where `argot fit` has already been run
# and an out-of-voice hunk (docs/demo/receipts.py) is staged. render.sh sets
# that up against the pinned FastAPI benchmark checkout, so the GIF shows the
# exact hit the README quotes.
#
# Rendered at 2× so it stays retina-crisp when the README shows it at ~760px.

Output demo.gif

Require argot

Set Shell "bash"
Set FontSize 34
Set Width 2000
# Sized to hug the output instead of a tall half-empty terminal.
Set Height 760
Set Padding 32
Set Margin 16
Set BorderRadius 12
Set Theme "Catppuccin Mocha"
Set Framerate 30
Set TypingSpeed 45ms

Sleep 600ms
Type "argot check --staged"
Sleep 500ms
Enter
Sleep 4800ms
