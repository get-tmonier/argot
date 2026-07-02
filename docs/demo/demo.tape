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
Set Height 760
Set Padding 24
Set Margin 12
Set BorderRadius 10
Set Theme "Catppuccin Mocha"
Set TypingSpeed 45ms

Sleep 600ms
Type "argot check --staged"
Sleep 700ms
Enter
Sleep 4500ms
